use eframe::egui::Vec2;
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSequence {
    bitmaps: Vec<Vec<bool>>,
    width: u8,
    height: u8,
}

impl ImageSequence {
    pub fn new(width: u8, height: u8) -> Self {
        Self {
            bitmaps: vec![vec![
                false;
                usize::from(width) * 8 * usize::from(height) * 8
            ]],
            width,
            height,
        }
    }

    pub fn get_frame_count(&self) -> usize {
        self.bitmaps.len()
    }

    pub fn get_dimensions(&self) -> [u8; 2] {
        [self.width, self.height]
    }

    pub fn get_dimensions_pixels(&self) -> [usize; 2] {
        [usize::from(self.width) * 8, usize::from(self.height) * 8]
    }

    pub fn get_dimensions_pixels_vec2(&self) -> Vec2 {
        let [width, height] = self.get_dimensions_pixels();
        Vec2::new(width as f32, height as f32)
    }

    pub fn get(&self, x: usize, y: usize, frame: usize) -> Option<&bool> {
        let [width_pixels, height_pixels] = self.get_dimensions_pixels();
        if (0..width_pixels).contains(&x) && (0..height_pixels).contains(&y) {
            self.bitmaps.get(frame)?.get(y * width_pixels + x)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize, frame: usize) -> Option<&mut bool> {
        let [width_pixels, height_pixels] = self.get_dimensions_pixels();
        if (0..width_pixels).contains(&x) && (0..height_pixels).contains(&y) {
            self.bitmaps.get_mut(frame)?.get_mut(y * width_pixels + x)
        } else {
            None
        }
    }

    pub fn get_frame(&self, frame: usize) -> Option<&[bool]> {
        self.bitmaps.get(frame).map(|vec| vec.as_ref())
    }

    pub fn iter_pixels(
        &self,
        frame: usize,
    ) -> Option<impl Iterator<Item = (usize, usize, bool)> + '_> {
        Some(
            self.get_frame(frame)?
                .iter()
                .enumerate()
                .map(|(i, &pixel)| {
                    let width = usize::from(self.width) * 8;
                    (i % width, i / width, pixel)
                }),
        )
    }

    pub fn get_bytes(&self, frame: usize) -> impl Iterator<Item = u8> + '_ {
        self.bitmaps[frame].chunks_exact(8).map(bits_to_byte)
    }

    pub fn add_frame(&mut self) {
        self.bitmaps.push(vec![
            false;
            usize::from(self.width)
                * 8
                * usize::from(self.height)
                * 8
        ]);
    }

    pub fn insert_frame(&mut self, idx: usize) {
        self.bitmaps.insert(
            idx,
            vec![false; usize::from(self.width) * 8 * usize::from(self.height) * 8],
        );
    }

    pub fn duplicate_frame(&mut self, idx: usize) {
        self.bitmaps.insert(idx + 1, self.bitmaps[idx].clone());
    }

    pub fn delete_frame(&mut self, idx: usize) {
        self.bitmaps.remove(idx);
    }

    pub fn clear_frame(&mut self, idx: usize) {
        self.bitmaps[idx]
            .iter_mut()
            .for_each(|pixel| *pixel = false);
    }

    pub fn get_frame_as_string(&self, idx: usize) -> String {
        let mut first = true;
        format!(
            "{{{}}}",
            self.get_bytes(idx)
                .fold(String::default(), |previous, current| {
                    if first {
                        first = false;
                        format!("{current:#04X}")
                    } else {
                        format!("{previous}, {current:#04X}")
                    }
                })
        )
    }

    pub fn get_sequence_as_string(&self) -> String {
        let mut first = true;
        format!(
            "{{{}}}",
            self.bitmaps
                .iter()
                .enumerate()
                .map(|(i, _)| self.get_frame_as_string(i))
                .fold(String::default(), |mut previous, current| {
                    if first {
                        first = false;
                        current
                    } else {
                        previous += ", ";
                        previous += &current;
                        previous
                    }
                })
        )
    }
}

impl Index<[usize; 3]> for ImageSequence {
    type Output = bool;

    fn index(&self, index: [usize; 3]) -> &Self::Output {
        self.get(index[0], index[1], index[2]).unwrap()
    }
}

impl IndexMut<[usize; 3]> for ImageSequence {
    fn index_mut(&mut self, index: [usize; 3]) -> &mut Self::Output {
        self.get_mut(index[0], index[1], index[2]).unwrap()
    }
}

fn bits_to_byte(bits: &[bool]) -> u8 {
    bits.iter().fold(0, |byte, &bit| byte << 1 | bit as u8)
}
