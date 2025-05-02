use pdf_font_parser::type1::Table;

fn main() {
    let file = include_bytes!("../font-0009.pfa");

    let table = Table::parse(&file[..]).unwrap();
}