//! The Ninetail wordmark above the sidebar's Home row, on Home.
//!
//! `assets/logo/ninetail.png` is the design's dot grid, one pixel per dot:
//! letters dense at the top that thin out toward the bottom. Scaled to the
//! bar, those dots would blur or shimmer, so the wordmark is dithered again
//! for the screen, one dot per physical pixel. Each letter keeps its edge,
//! and inside it the dots follow how densely the design fills that spot.
//!
//! The texture is white and painted in the theme's text colour, so it is
//! ink on a light theme and light on a dark one, as the design shows.

use std::sync::OnceLock;

use egui::{Color32, ColorImage, Rect, Sense, TextureHandle, TextureOptions, Vec2, pos2};

use crate::theme::Palette;

/// The wordmark's height on screen.
pub const HEIGHT: f32 = 20.0;

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
/// Each screen pixel is judged from this many samples a side.
const SUBSAMPLES: usize = 4;

/// The design's dots, with the letters' outlines and how densely each spot
/// is filled worked out from them.
struct Design {
    width: usize,
    height: usize,
    /// 1 inside a letter, 0 outside: the dots with the gaps between
    /// neighbours closed.
    shape: Vec<f32>,
    /// The share of a letter's dots lit around each dot, 0 to 1.
    density: Vec<f32>,
}

impl Design {
    fn load() -> &'static Self {
        static DESIGN: OnceLock<Design> = OnceLock::new();
        DESIGN.get_or_init(|| {
            let image = image::load_from_memory(include_bytes!("../../assets/logo/ninetail.png"))
                .expect("the bundled wordmark decodes")
                .to_luma_alpha8();
            let (width, height) = (image.width() as usize, image.height() as usize);
            let dots = image.pixels().map(|pixel| pixel[1] > 127).collect();
            Self::from_dots(width, height, dots)
        })
    }

    fn from_dots(width: usize, height: usize, dots: Vec<bool>) -> Self {
        let at = |grid: &[bool], x: isize, y: isize| {
            x >= 0
                && y >= 0
                && (x as usize) < width
                && (y as usize) < height
                && grid[y as usize * width + x as usize]
        };
        let around = |x: usize, y: usize| {
            (-1..=1).flat_map(move |dy| (-1..=1).map(move |dx| (x as isize + dx, y as isize + dy)))
        };
        // Closing: grow every dot by one, then shrink back, which fills the
        // dither's gaps but keeps each letter's outline.
        let grown: Vec<bool> = (0..width * height)
            .map(|index| around(index % width, index / width).any(|(x, y)| at(&dots, x, y)))
            .collect();
        let shape: Vec<bool> = (0..width * height)
            .map(|index| around(index % width, index / width).all(|(x, y)| at(&grown, x, y)))
            .collect();
        let density = (0..width * height)
            .map(|index| {
                let (x, y) = (index % width, index / width);
                let inside = around(x, y).filter(|&(x, y)| at(&shape, x, y)).count();
                let lit = around(x, y)
                    .filter(|&(x, y)| at(&shape, x, y) && at(&dots, x, y))
                    .count();
                lit as f32 / inside.max(1) as f32
            })
            .collect();
        Self {
            width,
            height,
            shape: shape
                .into_iter()
                .map(|inside| inside as u8 as f32)
                .collect(),
            density,
        }
    }

    /// The wordmark's width for a height of `rows` pixels.
    fn columns(&self, rows: usize) -> usize {
        (rows as f32 * self.width as f32 / self.height as f32).round() as usize
    }

    /// The wordmark `rows` pixels tall, white dots on transparent.
    fn raster(&self, rows: usize) -> ColorImage {
        let rows = rows.max(1);
        let columns = self.columns(rows).max(1);
        let scale_x = self.width as f32 / columns as f32;
        let scale_y = self.height as f32 / rows as f32;
        let mut pixels = vec![Color32::TRANSPARENT; columns * rows];
        for row in 0..rows {
            for column in 0..columns {
                let (mut shape, mut density) = (0.0, 0.0);
                for sy in 0..SUBSAMPLES {
                    for sx in 0..SUBSAMPLES {
                        let x = (column as f32 + (sx as f32 + 0.5) / SUBSAMPLES as f32) * scale_x;
                        let y = (row as f32 + (sy as f32 + 0.5) / SUBSAMPLES as f32) * scale_y;
                        let index = (y as usize).min(self.height - 1) * self.width
                            + (x as usize).min(self.width - 1);
                        shape += self.shape[index];
                        density += self.density[index] * self.shape[index];
                    }
                }
                if shape < (SUBSAMPLES * SUBSAMPLES) as f32 / 2.0 {
                    continue;
                }
                let threshold = (BAYER[row % 4][column % 4] as f32 + 0.5) / 16.0;
                if density / shape > threshold {
                    pixels[row * columns + column] = Color32::WHITE;
                }
            }
        }
        ColorImage::new([columns, rows], pixels)
    }
}

/// The wordmark's size in points at `pixels_per_point`, and its height in
/// physical pixels.
fn size(pixels_per_point: f32) -> (Vec2, usize) {
    let rows = (HEIGHT * pixels_per_point).round().max(1.0) as usize;
    let columns = Design::load().columns(rows);
    (
        Vec2::new(columns as f32, rows as f32) / pixels_per_point,
        rows,
    )
}

/// Draws the wordmark in the next space of `ui`'s layout.
pub fn show(ui: &mut egui::Ui, palette: &Palette) -> egui::Response {
    let ppp = ui.ctx().pixels_per_point();
    let (points, rows) = size(ppp);
    let (rect, response) = ui.allocate_exact_size(points, Sense::hover());
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Image);
        node.set_label("Ninetail");
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let id = egui::Id::new("ninetail-logo");
    let texture = ui
        .ctx()
        .data(|data| data.get_temp::<(usize, TextureHandle)>(id))
        .filter(|(made_for, _)| *made_for == rows)
        .map(|(_, texture)| texture)
        .unwrap_or_else(|| {
            let texture = ui.ctx().load_texture(
                "ninetail-logo",
                Design::load().raster(rows),
                TextureOptions::NEAREST,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, (rows, texture.clone())));
            texture
        });
    // On whole physical pixels, so each dot is exactly one.
    let min = pos2((rect.min.x * ppp).round(), (rect.min.y * ppp).round()) / ppp;
    ui.painter().image(
        texture.id(),
        Rect::from_min_size(min, points),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        palette.text,
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(image: &ColorImage) -> usize {
        image.pixels.iter().filter(|pixel| pixel.a() > 0).count()
    }

    #[test]
    fn the_bundled_design_is_the_wordmark_dot_grid() {
        let design = Design::load();
        assert_eq!((design.width, design.height), (249, 51));
        assert!(
            design
                .density
                .iter()
                .all(|value| (0.0..=1.0).contains(value))
        );
    }

    /// Every dot is one physical pixel, so the texture is made for the
    /// screen's scale, keeps the design's proportions, and is never empty.
    #[test]
    fn the_wordmark_is_dithered_for_each_screen_scale() {
        for ppp in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let (points, rows) = size(ppp);
            let image = Design::load().raster(rows);
            assert_eq!(image.size[1], rows);
            assert!((points.y - HEIGHT).abs() <= 0.5 / ppp, "{ppp}: {points:?}");
            assert!((image.size[0] as f32 / rows as f32 - 249.0 / 51.0).abs() < 0.05);
            let share = lit(&image) as f32 / image.pixels.len() as f32;
            assert!((0.1..0.6).contains(&share), "{ppp}: {share}");
            assert!(
                image
                    .pixels
                    .iter()
                    .all(|pixel| *pixel == Color32::TRANSPARENT || *pixel == Color32::WHITE)
            );
        }
    }

    /// As in the design, the letters are dense at the top and thin out
    /// toward the bottom.
    #[test]
    fn the_dots_thin_out_toward_the_bottom() {
        let image = Design::load().raster(52);
        let [columns, rows] = image.size;
        let band = |from: usize, to: usize| {
            image.pixels[from * columns..to * columns]
                .iter()
                .filter(|pixel| pixel.a() > 0)
                .count()
        };
        assert!(band(rows / 4, rows / 2) > band(rows * 3 / 4, rows));
    }
}
