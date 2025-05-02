use std::str::FromStr;

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum AssetType {
    Font,
    Image,
    AudioSource,
}

impl FromStr for AssetType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ttf" | "otf" => Ok(Self::Font),
            "hdr" | "exr" | "bmp" | "dds" | "ff" | "gif" | "ico" | "jpeg" | "jpg" | "ktx2"
            | "png" | "pnm" | "qoi" | "tga" | "tiff" | "webp" => Ok(Self::Image),
            "mp3" | "flac" | "oga" | "ogg" | "spx" | "wav" => Ok(Self::AudioSource),
            _ => Err(()),
        }
    }
}
