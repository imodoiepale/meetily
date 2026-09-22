// Encode 16 kHz mono f32 PCM as a WAV byte buffer for cloud STT uploads.
// Pattern follows OpenWhispr's audioManager multipart file field (WAV blob).

/// Minimum samples (~100 ms at 16 kHz) before a cloud provider is worth calling.
pub const MIN_CLOUD_SAMPLES: usize = 1600;

pub fn f32_pcm_to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_bytes: Vec<u8> = samples
        .iter()
        .flat_map(|s| {
            let clamped = s.clamp(-1.0, 1.0);
            let i = (clamped * i16::MAX as f32) as i16;
            i.to_le_bytes()
        })
        .collect();

    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let block_align = channels * bits_per_sample / 8;
    let byte_rate = sample_rate * u32::from(block_align);
    let chunk_size = 36 + data_bytes.len() as u32;

    let mut wav = Vec::with_capacity(44 + data_bytes.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&chunk_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data_bytes);
    wav
}
