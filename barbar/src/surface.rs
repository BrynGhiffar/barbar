use chrono::{DateTime, Local};
use fontdue::{Font, Metrics, layout::{CoordinateSystem, GlyphPosition, Layout, TextStyle}};

#[derive(Default)]
pub struct Color {
    b: u8,
    g: u8,
    r: u8,
    a: u8
}

impl Color {
    pub fn new() -> Self {
        Color::default()
    }

    pub fn black() -> Self {
        let mut color = Color::new();
        color.b = 0;
        color.g = 0;
        color.r = 0;
        color.a = 255;

        color
    }

    pub fn red() -> Self {
        let mut color = Color::new();
        color.b = 0;
        color.g = 0;
        color.r = 255;
        color.a = 255;

        color
    }

    pub fn half_a(mut self) -> Self {
        self.a = 122;
        self
    }

    pub fn as_buff(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }
}

pub struct BarSurface<'a> {
    pub canvas: &'a mut [u8],
    pub font: Font,
    pub width: usize,
    pub height: usize
}

impl<'a> BarSurface<'a> {
    pub fn background(&mut self, color: Color) {
        self.canvas.chunks_exact_mut(4)
            .for_each(|buff| buff.copy_from_slice(&color.as_buff()));
    }

    pub fn draw_char(&mut self, ch: char) {
        let (met, bitmap) = self.font.rasterize(ch, 15.0);
        for gy in 0..met.height {
            for gx in 0..met.width {
                let bitmap_idx = gy * met.width + gx;
                let gx = gx + 30;
                let gy = gy + 10;
                let canvas_idx = (gy * self.width + gx) * 4;

                let buff = &mut self.canvas[canvas_idx..(canvas_idx + 4)];
                if bitmap[bitmap_idx] != 0 {
                    buff[0] = bitmap[bitmap_idx];
                    buff[1] = bitmap[bitmap_idx];
                    buff[2] = bitmap[bitmap_idx];
                    buff[3] = 255;
                }
            }
        }
    }

    fn draw_single_char(&mut self, x1: usize, y1: usize, gpos: &GlyphPosition, met: &Metrics, bm: &[u8]) {
        for gy in 0..met.height {
            for gx in 0..met.width {
                let bitmap_idx = gy * met.width + gx;
                let gx = x1 + gx + (gpos.x as usize);
                let gy = y1 + gy + (gpos.y as usize);
                let canvas_idx = (gy * self.width + gx) * 4;

                let buff = &mut self.canvas[canvas_idx..(canvas_idx + 4)];
                if bm[bitmap_idx] != 0 {
                    buff[0] = bm[bitmap_idx];
                    buff[1] = bm[bitmap_idx];
                    buff[2] = bm[bitmap_idx];
                    buff[3] = 255;
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x1: usize, y1: usize, x2: usize, y2: usize, color: Color) {
        for x in x1..(x2+1) {
            for y in [y1, y2] {
                let canvas_idx = (y * self.width + x) * 4;
                let buff = &mut self.canvas[canvas_idx..(canvas_idx + 4)];
                buff.copy_from_slice(&color.as_buff());
            }
        }

        for x in [x1, x2] {
            for y in y1..(y2+1) {
                let canvas_idx = (y * self.width + x) * 4;
                let buff = &mut self.canvas[canvas_idx..(canvas_idx + 4)];
                buff.copy_from_slice(&color.as_buff());
            }
        }
    }

    pub fn draw_text(&mut self, x1: usize, y1: usize, text: &str) {
        let size = 11.0;
        let font = [&self.font];
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(&font, &TextStyle::new(text, size, 0));
        let glyphs: Vec<_> = text.chars().map(|c| self.font.rasterize(c, size)).collect();
        for (pos, (metrics, bm)) in layout.glyphs().iter().zip(glyphs.iter()) {
            self.draw_single_char(x1, y1, pos, metrics, bm);
        }
    }

    pub fn draw(&mut self) {
        let now: DateTime<Local> = Local::now();
        let formatted_dt = now.format("%H.%M | %A, %e %B %Y").to_string();
        // self.background(Color::black().half_a());
        self.draw_text(10, self.height / 2 - 6, &formatted_dt);
        // self.draw_rect(0, 0, 15, 15, Color::red());
    }
}

// pub struct TextSurface {
//     glyphs: Vec<(Metrics, Vec<u8>)>,
//     layout: Layout
// }
