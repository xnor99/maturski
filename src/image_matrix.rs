use crate::Direction;
use eframe::egui::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::{Add, Index, IndexMut, Mul};

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

    pub fn get_dimensions_pixels(&self) -> [usize; 2] {
        [usize::from(self.width) * 8, usize::from(self.height) * 8]
    }

    pub fn get_dimensions_pixels_vec2(&self) -> Vec2 {
        let [width, height] = self.get_dimensions_pixels();
        Vec2::new(width as f32, height as f32)
    }

    pub fn get(&self, x: usize, y: usize, idx: usize) -> Option<&bool> {
        let [width_pixels, height_pixels] = self.get_dimensions_pixels();
        if (0..width_pixels).contains(&x) && (0..height_pixels).contains(&y) {
            self.bitmaps.get(idx)?.get(y * width_pixels + x)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize, idx: usize) -> Option<&mut bool> {
        let [width_pixels, height_pixels] = self.get_dimensions_pixels();
        if (0..width_pixels).contains(&x) && (0..height_pixels).contains(&y) {
            self.bitmaps.get_mut(idx)?.get_mut(y * width_pixels + x)
        } else {
            None
        }
    }

    pub fn get_frame(&self, idx: usize) -> Option<&[bool]> {
        self.bitmaps.get(idx).map(|vec| vec.as_ref())
    }

    pub fn get_frame_mut(&mut self, idx: usize) -> Option<&mut [bool]> {
        self.bitmaps.get_mut(idx).map(|vec| &mut vec[..])
    }

    pub fn iter_pixels(
        &self,
        idx: usize,
    ) -> Option<impl Iterator<Item = (usize, usize, bool)> + '_> {
        Some(self.get_frame(idx)?.iter().enumerate().map(|(i, &pixel)| {
            let width = usize::from(self.width) * 8;
            (i % width, i / width, pixel)
        }))
    }

    pub fn iter_pixels_mut(&mut self, idx: usize) -> Option<impl Iterator<Item = &mut bool>> {
        Some(self.get_frame_mut(idx)?.iter_mut())
    }

    pub fn iter_frames(&self) -> impl Iterator<Item = &[bool]> {
        self.bitmaps.iter().map(|vector| vector.as_ref())
    }

    pub fn get_bytes(&self, idx: usize) -> impl Iterator<Item = u8> + '_ {
        self.bitmaps[idx].chunks_exact(8).map(bits_to_byte)
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

    pub fn move_up(&mut self, idx: usize) -> bool {
        if idx != 0 {
            self.bitmaps.swap(idx, idx - 1);
            true
        } else {
            false
        }
    }

    pub fn move_down(&mut self, idx: usize) -> bool {
        if idx != self.bitmaps.len() - 1 {
            self.bitmaps.swap(idx, idx + 1);
            true
        } else {
            false
        }
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

    pub fn slide(&mut self, idx: usize, direction: Direction, animation: SlideAnimation) {
        let dimension = match direction {
            Direction::Top | Direction::Bottom => self.height,
            Direction::Left | Direction::Right => self.width,
        } * 8;

        let vector = match direction {
            Direction::Top => IVec::new(0, -1),
            Direction::Left => IVec::new(-1, 0),
            Direction::Bottom => IVec::new(0, 1),
            Direction::Right => IVec::new(1, 0),
        };

        (0..dimension - 1).for_each(|_| self.duplicate_frame(idx));

        let [width, height] = [i16::from(self.width) * 8, i16::from(self.height) * 8];
        let current_frame = self.get_frame(idx).unwrap().to_owned();
        (0..dimension).rev().for_each(|i| {
            let scaled_vector = vector
                * match animation {
                    SlideAnimation::SlideIn => i16::from(dimension) - i16::from(i) - 1,
                    SlideAnimation::SlideOut => i16::from(i),
                };
            let frame_number = idx + usize::from(i);
            self.clear_frame(frame_number);
            (0..width * height)
                .map(|i| IVec::new(i % width, i / width))
                .for_each(|current_pixel| {
                    let IVec { x: new_x, y: new_y } = current_pixel + scaled_vector;
                    if (0..width).contains(&new_x) && (0..height).contains(&new_y) {
                        self[[
                            new_x.try_into().unwrap(),
                            new_y.try_into().unwrap(),
                            frame_number,
                        ]] = current_frame
                            [usize::try_from(current_pixel.y * width + current_pixel.x).unwrap()];
                    }
                });
        });
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

#[derive(Clone, Copy)]
struct IVec {
    x: i16,
    y: i16,
}

impl IVec {
    fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

impl Mul<i16> for IVec {
    type Output = IVec;

    fn mul(self, rhs: i16) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Add<IVec> for IVec {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Clone, Copy)]
pub enum SlideAnimation {
    SlideIn,
    SlideOut,
}

impl Display for SlideAnimation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SlideAnimation::SlideIn => "Slide in",
                SlideAnimation::SlideOut => "Slide out",
            }
        )
    }
}

impl SlideAnimation {
    pub fn iter() -> impl ExactSizeIterator<Item = Self> {
        [Self::SlideIn, Self::SlideOut].into_iter()
    }
}
