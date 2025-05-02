use std::str::FromStr;
use log::error;
use crate::parser::Stream;

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
        
        while let Some(token) = s.next_token() {
            match token {
                b"/FontInfo" => skip_dict(&mut s),
                b"/FontName" => skip_token(&mut s),
                b"/PaintType" => skip_token(&mut s),
                b"/FontType" => skip_token(&mut s),
                b"/FontBBox" => skip_token(&mut s),
                b"/UniqueID" => skip_token(&mut s),
                b"/Metrics" => skip_dict(&mut s),
                b"/StrokeWidth" => skip_token(&mut s),
                b"/FontMatrix" => font_matrix = Some(read_font_matrix(&mut s)),
                _ => {}
            }
        }
        
        println!("{:?}", font_matrix);
        
        Some(
            Self {
                data
            }
        )
    }
}

fn read_font_matrix(stream: &mut Stream) -> [f32; 6] {
    let mut entries = [0.0f32; 6];
    let mut idx = 0;
    
    // Skip '[';
    skip_token(stream);
    
    while let Some(token) = stream.next_token() {
        entries[idx] = f32::from_str(std::str::from_utf8(token).unwrap()).unwrap();
        
        idx += 1;
        if idx == 5 {
            break;
        }
    }
    
    // Skip `]`.
    skip_token(stream);
    
    entries
}

fn skip_dict(stream: &mut Stream) {
    stream.skip_until(b"begin", |b| matches!(b, b"end"));
}

fn skip_token(stream: &mut Stream) {
    stream.next_token();
}

trait StreamExt {
    fn next_token(&mut self) -> Option<&[u8]>;
    fn skip_line_comment(&mut self);
    fn skip_until(&mut self, find: &[u8], stop: impl Fn(&[u8]) -> bool);
}

impl StreamExt for Stream<'_> {
    fn next_token(&mut self) -> Option<&[u8]> {
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


#[cfg(test)]
mod tests {
    use crate::parser::Stream;
    use crate::type1::StreamExt;

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