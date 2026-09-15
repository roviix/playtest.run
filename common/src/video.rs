//! 首批视频检查：MP4 容器、至少一条 H.264 视频轨；音轨若存在只能是 AAC。
//!
//! 这里只解析元数据，不转码、不执行外部程序。解析是顺序读取，CLI 检查大文件时不会把整段
//! 视频放进内存；服务端仍需在接收边界再次执行同一契约，不能只信扩展名。

use std::io::Read;

use mp4parse::{CodecType, SampleEntry, TrackType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoInfo {
    pub video_tracks: usize,
    pub audio_tracks: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("this file is not a complete, readable MP4: {0}")]
    Invalid(String),
    #[error("this MP4 has no video track. A video project needs H.264 video, optionally with an AAC audio track.")]
    NoVideo,
    #[error("the video track is not H.264. Only H.264 inside an MP4 container is accepted, and nothing is transcoded behind your back.")]
    UnsupportedVideo,
    #[error("the audio track is not AAC. A video can be silent, but if it has sound, that sound has to be AAC.")]
    UnsupportedAudio,
}

pub fn inspect<R: Read>(reader: &mut R) -> Result<VideoInfo, VideoError> {
    let context =
        mp4parse::read_mp4(reader).map_err(|error| VideoError::Invalid(error.to_string()))?;
    let mut video_tracks = 0;
    let mut audio_tracks = 0;
    for track in context.tracks.iter() {
        match track.track_type {
            TrackType::Video => {
                video_tracks += 1;
                let valid = track.stsd.as_ref().is_some_and(|stsd| {
                    !stsd.descriptions.is_empty()
                        && stsd.descriptions.iter().all(|entry| {
                            matches!(entry, SampleEntry::Video(video) if video.codec_type == CodecType::H264)
                        })
                });
                if !valid {
                    return Err(VideoError::UnsupportedVideo);
                }
            }
            TrackType::Audio => {
                audio_tracks += 1;
                let valid = track.stsd.as_ref().is_some_and(|stsd| {
                    !stsd.descriptions.is_empty()
                        && stsd.descriptions.iter().all(|entry| {
                            matches!(entry, SampleEntry::Audio(audio) if audio.codec_type == CodecType::AAC)
                        })
                });
                if !valid {
                    return Err(VideoError::UnsupportedAudio);
                }
            }
            _ => {}
        }
    }
    if video_tracks == 0 {
        return Err(VideoError::NoVideo);
    }
    Ok(VideoInfo {
        video_tracks,
        audio_tracks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample(video: mp4::MediaConfig, audio: bool) -> Vec<u8> {
        let config = mp4::Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec!["isom".parse().unwrap(), "mp41".parse().unwrap()],
            timescale: 1000,
        };
        let mut writer = mp4::Mp4Writer::write_start(Cursor::new(Vec::new()), &config).unwrap();
        writer.add_track(&mp4::TrackConfig::from(video)).unwrap();
        if audio {
            writer
                .add_track(&mp4::TrackConfig::from(mp4::MediaConfig::AacConfig(
                    mp4::AacConfig::default(),
                )))
                .unwrap();
        }
        writer.write_end().unwrap();
        writer.into_writer().into_inner()
    }

    #[test]
    fn accepts_h264_with_optional_aac() {
        let bytes = sample(
            mp4::MediaConfig::AvcConfig(mp4::AvcConfig {
                width: 640,
                height: 360,
                seq_param_set: vec![0x67, 0x42, 0x00, 0x1e],
                pic_param_set: vec![0x68, 0xce, 0x06, 0xe2],
            }),
            true,
        );
        let info = inspect(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(info.video_tracks, 1);
        assert_eq!(info.audio_tracks, 1);
    }

    #[test]
    fn rejects_other_video_codecs_and_broken_files() {
        let bytes = sample(
            mp4::MediaConfig::HevcConfig(mp4::HevcConfig {
                width: 640,
                height: 360,
            }),
            false,
        );
        assert!(matches!(
            inspect(&mut Cursor::new(bytes)),
            Err(VideoError::UnsupportedVideo)
        ));
        assert!(matches!(
            inspect(&mut Cursor::new(b"not an mp4")),
            Err(VideoError::Invalid(_))
        ));
    }
}
