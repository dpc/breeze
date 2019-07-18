/// Renderer `Coord`-inate
///
/// This is logically different from the text `Coord`-inate,

#[derive(Copy, Clone, Debug, Default)]
pub struct Style {
    pub fg: u32,
    pub bg: u32,
    pub style: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Coord {
    pub x: usize,
    pub y: usize,
}

impl std::ops::Add<Coord> for Coord {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Coord {
            y: self.y + other.y,
            x: self.x + other.x,
        }
    }
}

impl Coord {
    fn is_inside(self, other: Coord) -> bool {
        self.y < other.y && self.x < other.x
    }
}

pub trait Renderer {
    fn dimensions(&self) -> Coord;
    fn rect(&self) -> Rect {
        Rect {
            offset: Coord { x: 0, y: 0 },
            dimensions: self.dimensions(),
        }
    }
    fn put(&mut self, coord: Coord, ch: char, style: Style);
    fn print(&mut self, coord: Coord, text: &str, style: Style) {
        let dims = self.dimensions();
        for (i, ch) in text.chars().enumerate() {
            if dims.y <= coord.y || dims.x <= (coord.x + i) {
                break;
            }
            self.put(coord, ch, style);
        }
    }
}

impl<T> Renderer for &mut T
where
    T: Renderer + ?Sized,
{
    fn dimensions(&self) -> Coord {
        (**self).dimensions()
    }
    fn put(&mut self, coord: Coord, ch: char, style: Style) {
        (**self).put(coord, ch, style)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub offset: Coord,
    pub dimensions: Coord,
}

impl Rect {
    pub fn split_verticaly_at(self, x: isize) -> (Rect, Rect) {
        let x = if x < 0 {
            self.dimensions.x + (-x as usize)
        } else if x > 0 {
            self.dimensions.x
        } else {
            panic!("Can't split at 0")
        };

        assert!(x < self.dimensions.x);
        (
            Rect {
                offset: self.offset,
                dimensions: Coord {
                    x,
                    y: self.dimensions.y,
                },
            },
            Rect {
                offset: Coord {
                    x: self.dimensions.x + x,
                    y: self.dimensions.y,
                },
                dimensions: Coord {
                    x: self.dimensions.x - x,
                    y: self.dimensions.y,
                },
            },
        )
    }

    pub fn split_horizontaly_at(self, y: isize) -> (Rect, Rect) {
        let y = if y < 0 {
            self.dimensions.y + (-y as usize)
        } else if y > 0 {
            self.dimensions.y
        } else {
            panic!("Can't split at 0")
        };

        assert!(y < self.dimensions.y);

        (
            Rect {
                offset: self.offset,
                dimensions: Coord {
                    x: self.dimensions.x,
                    y,
                },
            },
            Rect {
                offset: Coord {
                    x: self.dimensions.x,
                    y: self.dimensions.y + y,
                },
                dimensions: Coord {
                    x: self.dimensions.x,
                    y: self.dimensions.y - y,
                },
            },
        )
    }

    pub fn to_renderer<'r, R>(self, r: &'r mut R) -> View<'r, R>
    where
        R: Renderer,
    {
        View {
            view: self,
            backend: r,
        }
    }
}

pub struct View<'r, R> {
    view: Rect,
    backend: &'r mut R,
}

impl<'r, R> Renderer for View<'r, R>
where
    R: Renderer,
{
    fn dimensions(&self) -> Coord {
        self.view.dimensions
    }

    fn put(&mut self, coord: Coord, ch: char, style: Style) {
        if coord.is_inside(self.view.dimensions) {
            self.backend.put(coord + self.view.offset, ch, style)
        }
    }
}
