use pathtracer::math::{EnhancedVector, Vector3};

use glium::texture::*;

pub fn create_solid_color_texture(display: &glium::Display, color: (u8, u8, u8, u8)) -> Texture2d {
    let texture_data = RawImage2d::from_raw_rgba(vec![color.0, color.1, color.2, color.3], (1, 1));
    let format = UncompressedFloatFormat::U8U8U8U8;
    let mips = MipmapsOption::NoMipmap;

    Texture2d::with_format(display, texture_data, format, mips).unwrap()
}

pub struct TextureBlock {
    pub width: u32,
    pub height: u32,
    pub data: Vec<Vector3>,
}

struct VectorWrapper<'a> {
    data: &'a Vec<Vector3>,
    index: usize,
    sub_index: usize,
}

impl<'a> VectorWrapper<'a> {
    #[allow(dead_code)]
    pub fn new(data: &'a Vec<Vector3>) -> Self {
        Self {
            data,
            index: 0,
            sub_index: 0,
        }
    }
}

impl<'a> Iterator for VectorWrapper<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.data.len() {
            return None;
        }

        if self.sub_index == 3 {
            self.index += 1;
            self.sub_index = 0;
            Some(1.0)
        } else {
            let val = self.data[self.index][self.sub_index];
            self.sub_index += 1;
            Some(val)
        }
    }
}

impl TextureBlock {
    pub fn new(width: u32, height: u32) -> TextureBlock {
        TextureBlock {
            width,
            height,
            data: vec![Vector3::zero(); (width * height) as usize],
        }
    }

    #[inline]
    fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32, color: Vector3) {
        let index = self.index(x, y);
        self.data[index] = color;
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> &Vector3 {
        &self.data[self.index(x, y)]
    }
}

pub struct TextureData {
    pub texture: Texture2d,
    pub data: Vec<Vector3>,
    pub dimensions: (u32, u32),
}

impl TextureData {
    pub fn new(display: &glium::Display, width: u32, height: u32) -> TextureData {
        let dimensions = (width, height);
        let data = vec![Vector3::zero(); (width * height * 4) as usize];

        let format = UncompressedFloatFormat::F32F32F32F32;
        let mipmaps = MipmapsOption::NoMipmap;

        TextureData {
            texture: glium::Texture2d::empty_with_format(display, format, mipmaps, width, height)
                .unwrap(),
            data,
            dimensions,
        }
    }

    #[allow(dead_code)]
    pub fn set_data(&mut self, data: Vec<Vector3>) {
        assert_eq!(self.data.len(), data.len());

        self.data = data;
        self.reset();
    }

    pub fn reset(&mut self) {
        let data = VectorWrapper::new(&self.data).collect::<Vec<f32>>();

        assert_eq!(self.data.len() * 4, data.len());
        self.sync(data, (0, 0, self.dimensions.0, self.dimensions.1));
    }

    pub fn sync(&mut self, data: Vec<f32>, rect: (u32, u32, u32, u32)) {
        let raw = RawImage2d::from_raw_rgba(data, (rect.2, rect.3));
        let rect = glium::Rect {
            left: rect.0,
            bottom: rect.1,
            width: rect.2,
            height: rect.3,
        };

        self.texture.write(rect, raw);
    }

    pub fn clear(&mut self) {
        self.data.fill(Vector3::zero());
        self.sync(
            vec![0.; (self.dimensions.0 * self.dimensions.1 * 4) as usize],
            (0, 0, self.dimensions.0, self.dimensions.1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::VectorWrapper;
    use pathtracer::math::Vector3;

    #[test]
    fn test_vector_wrapper() {
        let data = vec![
            Vector3::new(1., 2., 3.),
            Vector3::new(4., 5., 6.),
            Vector3::new(7., 8., 9.),
        ];

        let converted: Vec<f32> = VectorWrapper::new(&data).collect();

        assert_eq!(converted.len(), 4 * 3);
        assert_eq!(converted[0], 1.);
        assert_eq!(converted[1], 2.);
        assert_eq!(converted[2], 3.);
        assert_eq!(converted[3], 1.);

        assert_eq!(converted[4], 4.);
        assert_eq!(converted[5], 5.);
        assert_eq!(converted[6], 6.);
        assert_eq!(converted[7], 1.);

        assert_eq!(converted[8], 7.);
        assert_eq!(converted[9], 8.);
        assert_eq!(converted[10], 9.);
        assert_eq!(converted[11], 1.);
    }
}
