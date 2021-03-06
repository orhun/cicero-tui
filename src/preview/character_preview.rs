// This file is part of Cicero.
//
// Cicero is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// Cicero is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
// A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// Cicero. If not, see <https://www.gnu.org/licenses/>.

use std::cmp::min;

use freetype::{Face, Library};

use super::font_match::fonts_for;
use super::stateful_vec::StatefulVec;
use super::{Error, Result};

#[derive(Debug, Copy, Clone)]
pub struct RenderSize {
    pub width: usize,
    pub height: usize,
}

impl RenderSize {
    pub fn new(width: usize, height: usize) -> Self {
        RenderSize { width, height }
    }
}

#[derive(Debug)]
pub struct RenderedCharacter {
    pub bitmap: Vec<Vec<u8>>, // TODO: This naive 2D vector is not really optimized
    pub glyph_size: RenderSize, // TODO: Expose all glyph metrics
}

pub struct CharacterPreview {
    pub chr: char,

    paths_for_matching_fonts: StatefulVec<String>,

    library: Library, // TODO: Make this a long-living object to avoid re-init it for each character
    current_font: Face,
}

impl CharacterPreview {
    pub fn new(chr: char, preferred_font_path: Option<&String>) -> Result<CharacterPreview> {
        let font_paths = fonts_for(chr)?;
        if font_paths.is_empty() {
            return Err(Box::new(Error::GlyphNotFound { chr }));
        }

        let mut paths_for_matching_fonts = StatefulVec::new(font_paths, Some(0));
        if let Some(font_path) = preferred_font_path {
            paths_for_matching_fonts.select_if_found(font_path);
        }

        let library = Library::init()?;
        let current_font =
            library.new_face(&paths_for_matching_fonts.current_item().unwrap(), 0)?;

        Ok(CharacterPreview {
            chr,
            paths_for_matching_fonts,
            library,
            current_font,
        })
    }

    pub fn get_current_font_path(&self) -> Option<String> {
        match self.paths_for_matching_fonts.current_item() {
            Some(current_font_path) => Some(current_font_path.to_owned()),
            None => None,
        }
    }

    pub fn has_previous_font(&self) -> bool {
        self.paths_for_matching_fonts.has_previous()
    }

    pub fn select_previous_font(&mut self) -> Result<()> {
        self.paths_for_matching_fonts.select_previous();
        self.current_font = match self.paths_for_matching_fonts.current_item() {
            Some(current_font_path) => self.library.new_face(current_font_path, 0)?,
            None => return Err(Box::new(Error::GlyphNotFound { chr: self.chr })),
        };
        Ok(())
    }

    pub fn has_next_font(&self) -> bool {
        self.paths_for_matching_fonts.has_next()
    }

    pub fn select_next_font(&mut self) -> Result<()> {
        self.paths_for_matching_fonts.select_next();
        self.current_font = match self.paths_for_matching_fonts.current_item() {
            Some(current_font_path) => self.library.new_face(current_font_path, 0)?,
            None => return Err(Box::new(Error::GlyphNotFound { chr: self.chr })),
        };
        Ok(())
    }

    pub fn get_current_font_display_name(&self) -> String {
        let family_name = self
            .current_font
            .family_name()
            .unwrap_or_else(|| "Unknown Family".to_owned());
        let style_name = self
            .current_font
            .style_name()
            .unwrap_or_else(|| "Unknown Style".to_owned());
        format!("{} - {}", family_name, style_name)
    }

    pub fn render(&self, size: RenderSize) -> Result<RenderedCharacter> {
        self.current_font
            .set_pixel_sizes(size.width as u32, size.height as u32)?;
        self.current_font
            .load_char(self.chr as usize, freetype::face::LoadFlag::RENDER)?;

        let (bitmap, glyph_size) = {
            let mut pixels = vec![vec![0; size.width as usize]; size.height as usize];

            let glyph_bitmap = self.current_font.glyph().bitmap();
            let x_max = min(size.width, glyph_bitmap.width() as usize);
            let y_max = min(size.height, glyph_bitmap.rows() as usize);

            let glyph_bitmap_buffer = glyph_bitmap.buffer();

            for x in 0..x_max {
                for y in 0..y_max {
                    pixels[y][x] = glyph_bitmap_buffer[y * x_max + x];
                }
            }

            (pixels, RenderSize::new(x_max, y_max))
        };

        Ok(RenderedCharacter { bitmap, glyph_size })
    }
}
