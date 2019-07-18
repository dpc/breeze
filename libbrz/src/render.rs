/// Renderer `Coord`-inate
///
/// This is logically different from the text `Coord`-inate,

#[derive(Copy, Clone, Debug)]
pub struct Style {
    fg: u32,
    bg: u32,
    style: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Coord {
    line: usize,
    column: usize,
}

impl std::ops::Add<Coord> for Coord {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Coord {
            column: self.column + other.column,
            line: self.line + other.line,
        }
    }
}

impl Coord {
    fn is_inside(self, other: Coord) -> bool {
        self.line <= other.line && self.column <= other.line
    }
}

pub trait Renderer {
    fn size(&self) -> Coord;
    fn put(&mut self, coord: Coord, ch: char, style: Style);
    fn print(&mut self, coord: Coord, text: &str, style: Style) {
        let size = self.size();
        for (i, ch) in text.chars().enumerate() {
            if size.line < coord.line || size.column < (coord.column + i) {
                break;
            }
            self.put(coord, ch, style);
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct View {
    offset: Coord,
    size: Coord,
}

impl View {
    fn render_to<'r, R>(self, r: &'r mut R) -> ViewRenderer<'r, R>
    where
        R: Renderer,
    {
        ViewRenderer {
            view: self,
            backend: r,
        }
    }
}

struct ViewRenderer<'r, R> {
    view: View,
    backend: &'r mut R,
}

impl<'r, R> Renderer for ViewRenderer<'r, R>
where
    R: Renderer,
{
    fn size(&self) -> Coord {
        self.view.size
    }

    fn put(&mut self, coord: Coord, ch: char, style: Style) {
        if coord.is_inside(self.view.size) {
            self.backend.put(coord + self.view.offset, ch, style)
        }
    }
}
