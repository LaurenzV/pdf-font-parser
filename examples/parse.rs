use log::trace;
use pdf_font_parser::type1::Table;
use pdf_font_parser::OutlineBuilder;

fn main() {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Trace);
    }

    let file1 = include_bytes!("../font-0009.pfa");

    let mut out = DummyOutline;

    for file in [&file1[..]] {
        // for file in [&file1[..]] {
        let table = Table::parse(&file[..]).unwrap();

        for c in 0..=255 {
            table.code_to_string(c).map(|c| {
                trace!("Outlining {}", c);
                table.outline(c, &mut out)
            });
        }
    }
}

struct DummyOutline;

impl OutlineBuilder for DummyOutline {
    fn move_to(&mut self, x: f32, y: f32) {}

    fn line_to(&mut self, x: f32, y: f32) {}

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {}

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {}

    fn close(&mut self) {}
}

/// A simple stderr logger.
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Trace
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let target = if record.target().len() > 0 {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let line = record.line().unwrap_or(0);
            let args = record.args();

            match record.level() {
                log::Level::Error => eprintln!("Error (in {}:{}): {}", target, line, args),
                log::Level::Warn => eprintln!("Warning (in {}:{}): {}", target, line, args),
                log::Level::Info => eprintln!("Info (in {}:{}): {}", target, line, args),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, args),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, args),
            }
        }
    }

    fn flush(&self) {}
}
