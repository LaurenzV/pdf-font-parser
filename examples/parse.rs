use pdf_font_parser::type1::Table;

fn main() {
    let file1 = include_bytes!("../font-0009.pfa");
    let file2 = include_bytes!("../font-0011.pfa");
    let file3 = include_bytes!("../font-0013.pfa");

    for file in [&file1[..], &file2[..], &file3[..]] {
        // for file in [&file1[..]] {
        println!("new file");
        let table = Table::parse(&file[..]).unwrap();
    }
}
