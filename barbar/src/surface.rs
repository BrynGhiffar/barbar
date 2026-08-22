use chrono::{DateTime, Local};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};

use crate::stat::{BarStat, Unit};

#[derive(Default, Clone, Copy)]
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

    pub fn white() -> Self {
        let mut color = Color::new();
        color.b = 255;
        color.g = 255;
        color.r = 255;
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

    pub fn lerp_a(mut self, lo: usize, hi: usize, curr: usize) -> Self {
        let d = 1.0 - (curr - lo) as f32 / (hi - lo) as f32;
        self.a = (d * 180_f32) as u8;
        self
    }

    pub fn half_a(mut self) -> Self {
        self.a = 122;
        self
    }

    pub fn as_buff(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }
}

const BPP: usize = 4;

pub struct SurfaceView<'a> {
    canvas: &'a mut [u8],
    width: usize,
    height: usize,
    stride: usize
}

impl<'a> SurfaceView<'a> {
    pub fn from_raw(canvas: &'a mut [u8], width: usize, height: usize) -> Self {
        SurfaceView { canvas, width, height, stride: width }
    }

    pub fn fill(&mut self, color: Color) {
        let mut current = &mut self.canvas[..];
        for _ in 0..self.height {
            current[..self.width * BPP]
                .chunks_exact_mut(BPP).for_each(|chunk| chunk.copy_from_slice(&color.as_buff()));
            current = &mut current[self.stride * BPP..];
        }
    }

    pub fn fill_fade(&mut self, color: Color) {
        let mut current = &mut self.canvas[..];
        for h in 0..self.height {
            current[..self.width * BPP]
                .chunks_exact_mut(BPP).for_each(|chunk| chunk.copy_from_slice(&color.lerp_a(0, self.height, h).as_buff()));
            current = &mut current[self.stride * BPP..];
        }
    }

    #[inline]
    pub fn sub_view(
        &'a mut self,
        x: usize,
        y: usize,
        view_width: usize,
        view_height: usize
    ) -> Option<SurfaceView<'a>> {
        if x + view_width > self.width || y + view_height > self.height {
            return None;
        }

        let start_byte = (y * self.stride + x) * BPP;
        let required_bytes = if view_height == 0 { 0 } else {
            ((view_height - 1) * self.stride + view_width) * BPP
        };

        Some(SurfaceView {
            canvas: &mut self.canvas[start_byte..start_byte + required_bytes],
            width: view_width,
            height: view_height,
            stride: self.stride
        })
    }

    #[inline]
    pub fn sub_view_margin(
        &'a mut self,
        margin_y: usize,
        margin_x: usize
    ) -> Option<SurfaceView<'a>> {
        self.sub_view(margin_x, margin_y, self.width - margin_x * 2, self.height - margin_y * 2)
    }

    #[inline]
    pub fn sub_view_center_y(
        &'a mut self,
        view_height: usize,
    ) -> Option<SurfaceView<'a>> {
        self.sub_view(0, self.height / 2 - view_height / 2 - 1, self.width, view_height)
    }

    #[inline]
    pub fn sub_view_align_right(
        &'a mut self,
        view_width: usize,
    ) -> Option<SurfaceView<'a>> {
        self.sub_view(self.width - view_width, 0, view_width, self.height)
    }

    pub fn draw_glyph(
        &mut self,
        x: isize,            // Logical X on canvas (can be negative due to glyph bearing)
        y: isize,            // Logical Y on canvas (can be negative)
        metrics: &fontdue::Metrics,   // fontdue::Metrics
        bitmap: &[u8],       // fontdue rasterized coverage buffer
        color: [u8; 4],      // Text color in BGRA format: [B, G, R, A]
    ) {
        if metrics.width == 0 || metrics.height == 0 || color[3] == 0 {
            return;
        }

        // 1. Calculate clipping bounds to handle partially visible glyphs safely
        let start_x = x.max(0) as usize;
        let start_y = y.max(0) as usize;
        let end_x = (x + metrics.width as isize).min(self.width as isize).max(0) as usize;
        let end_y = (y + metrics.height as isize).min(self.height as isize).max(0) as usize;

        if start_x >= end_x || start_y >= end_y {
            return; // Glyph is completely off-screen
        }

        // 2. Render clipped region
        for cy in start_y..end_y {
            let gy = (cy as isize - y) as usize;
            
            // CRITICAL: Use self.stride instead of self.width to respect sub-views!
            let canvas_row_start = cy * self.stride * 4;
            let bitmap_row_start = gy * metrics.width;

            for cx in start_x..end_x {
                let gx = (cx as isize - x) as usize;
                let coverage = bitmap[bitmap_row_start + gx];

                if coverage == 0 {
                    continue; // Skip transparent background pixels
                }

                // Modulate text color alpha with fontdue coverage intensity
                let alpha = (color[3] as u16 * coverage as u16) / 255;
                if alpha == 0 {
                    continue;
                }

                let canvas_idx = canvas_row_start + (cx * 4);
                let pixel = &mut self.canvas[canvas_idx..canvas_idx + 4];

                if alpha == 255 {
                    // Fast path for fully opaque pixel interior
                    pixel[0] = color[0]; // B
                    pixel[1] = color[1]; // G
                    pixel[2] = color[2]; // R
                    pixel[3] = 255;
                } else {
                    // Alpha blend anti-aliased font edges over existing background
                    let inv_alpha = 255 - alpha;
                    pixel[0] = ((color[0] as u16 * alpha + pixel[0] as u16 * inv_alpha) / 255) as u8; // B
                    pixel[1] = ((color[1] as u16 * alpha + pixel[1] as u16 * inv_alpha) / 255) as u8; // G
                    pixel[2] = ((color[2] as u16 * alpha + pixel[2] as u16 * inv_alpha) / 255) as u8; // R
                    pixel[3] = 255; // Keep target fully opaque
                }
            }
        }
    }

    pub fn draw_glyph_premultiplied(
        &mut self,
        x: isize,
        y: isize,
        metrics: &fontdue::Metrics,
        bitmap: &[u8],
        color: [u8; 4], // Expects Straight BGRA
    ) {
        // 1. Pre-multiply text color once per glyph
        let src_a = color[3] as u32;
        let src_b = (color[0] as u32 * src_a) / 255;
        let src_g = (color[1] as u32 * src_a) / 255;
        let src_r = (color[2] as u32 * src_a) / 255;

        // 1. Calculate clipping bounds to handle partially visible glyphs safely
        let start_x = x.max(0) as usize;
        let start_y = y.max(0) as usize;
        let end_x = (x + metrics.width as isize).min(self.width as isize).max(0) as usize;
        let end_y = (y + metrics.height as isize).min(self.height as isize).max(0) as usize;

        if start_x >= end_x || start_y >= end_y {
            return; // Glyph is completely off-screen
        }

        // ... [clipping and loop setup] ...

        for cy in start_y..end_y {
            let gy = (cy as isize - y) as usize;
            
            // CRITICAL: Use self.stride instead of self.width to respect sub-views!
            let canvas_row_start = cy * self.stride * 4;
            let bitmap_row_start = gy * metrics.width;
            for cx in start_x..end_x {
                let gx = (cx as isize - x) as usize;
                let coverage = bitmap[bitmap_row_start + gx] as u32;
                if coverage == 0 { continue; }

                // Scale premultiplied color by glyph coverage
                let sa = (src_a * coverage) / 255;
                let sb = (src_b * coverage) / 255;
                let sg = (src_g * coverage) / 255;
                let sr = (src_r * coverage) / 255;

                let inv_sa = 255 - sa;
                let canvas_idx = canvas_row_start + (cx * 4);
                let pixel = &mut self.canvas[canvas_idx..canvas_idx + 4];

                // Premultiplied OVER formula: Dst = Src + Dst * (1 - SrcAlpha)
                pixel[0] = (sb + (pixel[0] as u32 * inv_sa) / 255) as u8;
                pixel[1] = (sg + (pixel[1] as u32 * inv_sa) / 255) as u8;
                pixel[2] = (sr + (pixel[2] as u32 * inv_sa) / 255) as u8;
                pixel[3] = (sa + (pixel[3] as u32 * inv_sa) / 255) as u8;
            }
        }
    }

    pub fn text(&mut self, x: usize, y: usize, color: Color, font: &fontdue::Font , text: &str, size: f32) {
        // 1. Configure the layout engine
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(&[font], &TextStyle::new(text, size, 0));

        // layout

        // 2. Iterate through layout-calculated glyph positions
        for pos in layout.glyphs() {
            // pos.x and pos.y give the top-left offset of the glyph bitmap.
            // Round to nearest integer pixel coordinate.
            let glyph_x = (x as f32 + pos.x).round() as isize;
            let glyph_y = (y as f32 + pos.y).round() as isize;

            // 3. Rasterize using the exact key stored in GlyphPosition.
            // This properly accounts for font sizing and subpixel positioning.
            let (metrics, bitmap) = font.rasterize_config(pos.key);

            // 4. Render into our sub-view safe canvas
            // self.draw_glyph(glyph_x, glyph_y, &metrics, &bitmap, color.as_buff());
            self.draw_glyph_premultiplied(glyph_x, glyph_y, &metrics, &bitmap, color.as_buff());
        }
    }
}

pub fn calculate_width_height(font: &fontdue::Font, text: &str, size: f32) -> (f32, f32) {
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.append(&[font], &TextStyle::new(text, size, 0));
    
    let mut max_width = 0.0f32;
    let mut max_height = 0.0f32;
    for pos in layout.glyphs() {
        let right_edge = pos.x + pos.width as f32;
        let bottom_edge = pos.y + pos.height as f32;
        if right_edge > max_width {
            max_width = right_edge;
        }
        if bottom_edge > max_height {
            max_height = bottom_edge;
        }
    }

    (max_width, max_height)
}

pub struct BarSurface<'a> {
    root_view: SurfaceView<'a>,
}

impl<'a> BarSurface<'a> {
    pub fn from_raw(canvas: &'a mut [u8], width: usize, height: usize) -> Self {
        let root_view = SurfaceView::from_raw(canvas, width, height);

        Self { root_view }
    }

    pub fn draw(&'a mut self, font: &fontdue::Font, stat: Option<BarStat>) {
        let text_size = 12.0;
        let now: DateTime<Local> = Local::now();
        self.root_view.fill(Color::black().half_a());
        let Some(stat) = stat else {
            return;
        };
        let disk = format!("[/] {:.1}GB/{:.1}GB {:.0}%",
            stat.disk_used(Unit::GB),
            stat.disk_total(Unit::GB),
            stat.disk_used_pct()
        );
        let mem = format!("{:.0}% ", stat.mem_usage_pct());
        let cpu = format!("{:.0}% {}C ", stat.cpu_usage_pct(), stat.cpu_temp());
        let datetime = now.format("%H.%M.%S | %A, %e %B %Y").to_string();
        let template = format!("{disk} | {cpu} | {mem} | {datetime}");
        let (width, height) = calculate_width_height(font, &template, text_size);
        if let Some(mut comp_view) = self.root_view.sub_view_margin(0, 15) 
            && let Some(mut comp_view) = comp_view.sub_view_center_y(height as usize)
            && let Some(mut comp_view) = comp_view.sub_view_align_right(width as usize)
        {
            comp_view.text(0, 0, Color::white(), font, &template, text_size);
        }
    }
}
