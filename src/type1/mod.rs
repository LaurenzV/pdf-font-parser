mod decrypt;

use std::collections::HashMap;
use std::str::FromStr;
use log::error;
use crate::parser::Stream;
use crate::type1::decrypt::decrypt;
// Many parts of the parser code are adapted from
// https://github.com/janpe2/CFFDump/blob/master/cff/type1/Type1Dump.java

pub struct Table<'a> {
    data: &'a [u8]
}

impl<'a> Table<'a> {
    /// Parses a table from raw data.
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if !data.starts_with(b"%!") {
            error!("type1 font didn't start with %!");
            
            return None;
        }

        let mut s = Stream::new(data);
        
        let mut font_matrix = None;
        let mut encoding = None;
        
        while let Some(token) = s.next_token() {
            match token {
                b"/FontInfo" => s.skip_dict(),
                b"/FontName" => s.skip_token(),
                b"/PaintType" => s.skip_token(),
                b"/FontType" => s.skip_token(),
                b"/FontBBox" => s.skip_token(),
                b"/UniqueID" => s.skip_token(),
                b"/Metrics" => s.skip_dict(),
                b"/StrokeWidth" => s.skip_token(),
                b"/FontMatrix" => font_matrix = Some(s.read_font_matrix()),
                b"/Encoding" => encoding = Some(s.read_encoding()),
                b"eexec" => {
                    let decrypted = decrypt(s.tail().unwrap());
                    println!("{:?}", std::str::from_utf8(&decrypted[0..12]));
                }
                b"/Private" => {
                    println!("reached private dict");
                }
                _ => {}
            }
        }
        
        println!("{:?}", font_matrix);
        println!("{:?}", encoding);
        
        Some(
            Self {
                data
            }
        )
    }
}

impl<'a> Stream<'a> {
    fn peek_token(&mut self) -> Option<&'a [u8]> {
        self.clone().next_token()
    }
    
    fn next_token(&mut self) -> Option<&'a [u8]> {
        let mut skip_token = |st: &mut Stream| -> usize {
            let mut count = 1;
            while let Some(ch) = st.read_bytes(1) {
                if is_whitespace(ch[0]) || is_self_delim_after_token(ch[0])  {
                    st.move_back(1);
                    break;
                }
                
                count += 1;
            }
            
            count
        };
        
        while let Some(ch) = self.clone().read_bytes(1) {
            let tail = self.tail()?;
            self.read_bytes(1);
            
            if is_whitespace(ch[0]) {
                continue;
            }
            
            match ch[0] {
                b'%' => self.skip_line_comment(),
                b'(' => return Some(b"("),
                b'<' => {
                    if let Some(ch2) = self.read_bytes(1) {
                        if ch2[0] == b'>' {
                            return Some(b"( )")
                        }   else if ch2[0] == b'<' {
                            return Some(b"<<");
                        }   else {
                            return Some(b"<")
                        }
                    }
                }
                b'>' => {
                    if let Some(ch2) = self.read_bytes(1) {
                        if ch2[0] == b'>' {
                            return Some(b">>")
                        }   else {
                            self.move_back(1);
                            return Some(b">")
                        }
                    }
                }
                b'[' => {
                    return Some(b"[");
                }
                b']' => {
                    return Some(b"]");
                }
                b'{' => {
                    return Some(b"{");
                }
                b'}' => {
                    return Some(b"}");
                }
                b'/' => {
                    if let Some(ch2) = self.read_bytes(1) {
                        if is_whitespace(ch2[0]) || is_self_delim_after_token(ch2[0]) {
                            let token = b"/";
                            
                            if is_self_delim_after_token(ch2[0]) {
                                self.move_back(1);
                            }
                            
                            return Some(token)
                        }   else {
                            let count = skip_token(self);
                            
                            return Some(&tail[0..(count + 1)])
                        }
                    }
                }
                _ => {
                    let count = skip_token(self);
                    return Some(&tail[0..count])
                }
            }
        }
        
        None
    }

    fn read_font_matrix(&mut self) -> [f32; 6] {
        let mut entries = [0.0f32; 6];
        let mut idx = 0;

        // Skip '[';
        self.skip_token();

        while let Some(token) = self.next_token() {
            entries[idx] = f32::from_str(std::str::from_utf8(token).unwrap()).unwrap();

            idx += 1;
            if idx == 5 {
                break;
            }
        }

        // Skip `]`.
        self.skip_token();

        entries
    }
    
    fn read_encoding(&mut self) -> EncodingType {
        let mut map = HashMap::new();
        
        let t1 = self.next_token().unwrap();
        let t2 = self.next_token().unwrap();
        
        if t1 == b"StandardEncoding" && t2 == b"def" {
            return EncodingType::Standard;
        }
        
        if !self.skip_until_before(b"dup", |b| matches!(b, b"def" | b"readonly")) {
            return EncodingType::Custom(map);
        }
        
        while let Some(token) = self.next_token() {
            if matches!(token, b"def" | b"readonly") {
                break;
            }
            
            if token != b"dup" {
                panic!("Unexpected token {:?}", token);
            }
            
            let code = u8::from_str(std::str::from_utf8(self.next_token().unwrap()).unwrap()).unwrap();
            let glyph_name = std::str::from_utf8(&self.next_token().unwrap()[1..]).unwrap().to_string();
            
            if self.next_token().unwrap() != b"put" {
                panic!("Unexpected token {:?}", token);
            }
            
            map.insert(code, glyph_name);
        }
        
        EncodingType::Custom(map)
    }

    fn skip_dict(&mut self) {
        self.skip_until(b"begin", |b| matches!(b, b"end"));
    }

    fn skip_token(&mut self) {
        self.next_token();
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.read::<u8>() {
            if matches!(ch, b'\n' | b'\r') {
                break;
            }
        }
    }

    fn skip_until(&mut self, find: &[u8], stop: impl Fn(&[u8]) -> bool) -> bool {
        while let Some(token) = self.next_token() {
            if token == find {
                return true;
            }
            
            if stop(token) {
                break;
            }
        }
        
        false
    }
    
    fn skip_whitespaces(&mut self) {
        while let Some(ch) = self.peek::<u8>() {
            if is_whitespace(ch) {
                self.read::<u8>();
            }   else {
                break;
            }
        }
    }

    fn skip_until_before(&mut self, find: &[u8], stop: impl Fn(&[u8]) -> bool) -> bool {
        while let Some(token) = self.peek_token() {
            if token == find {
                return true;
            }
            
            self.next_token().unwrap();
            
            if stop(token) {
                break;
            }
        }
        
        false
    }
}

fn is_whitespace(c: u8) -> bool {
    if c <= 32 {
        return matches!(c, b' ' | b'\n' | b'\r' | b'\t' | 0x00 | 0x0C);
    }
    
    false
}

fn is_self_delim_after_token(c: u8) -> bool {
    // The characters ()<>[]{}/% are special. They delimit syntactic entities
    // such as strings, procedure bodies, name literals, and comments. Any of these
    // characters terminates the entity preceding it and is not included in the entity.

    matches!(c, b'(' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' | b')')

    // This checks for self delimiters appearing after tokens. Thus there is no
    // need to check for ')'. However, char '>' can appear in keyword >>, like
    // here: /Pages 2 0 R>>. So the char '>' must end the token R.
}

#[derive(Debug)]
enum EncodingType {
    Standard,
    Custom(HashMap<u8, String>)
}


#[cfg(test)]
mod tests {
    use crate::parser::Stream;

    macro_rules! assert_token {
        ($content:expr, $token:expr) => {
            assert_eq!($content.next_token(), Some(&$token[..]))
        }
    }
    
    #[test]
    fn lexing_1() {
        let mut content = Stream::new(b"/FontInfo ");
        
        assert_token!(content, b"/FontInfo");
    }
    
    #[test]
    fn lexing_2() {
        let mut content = Stream::new(b"/version (01) readonly def");
        
        assert_token!(content, b"/version");
        assert_token!(content, b"(");
        assert_token!(content, b"01");
        assert_token!(content, b")");
        assert_token!(content, b"readonly");
        assert_token!(content, b"def");
    }

    
}