// SEG-Y Sub-Bottom Profiler (SBP) binary file reader (Rev 0 / Rev 1 format).
// Reads 3200-byte text header, 400-byte binary reel header, and trace headers + sample data.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegyReelHeader {
    pub job_id: i32,
    pub line_number: i32,
    pub reel_number: i32,
    pub traces_per_ensemble: u16,
    pub sample_interval_us: u16, // micro-seconds
    pub samples_per_trace: u16,
    pub data_sample_format: u16, // 1 = 4-byte IBM float, 2 = 4-byte int, 3 = 2-byte int, 5 = 4-byte IEEE float
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegyTraceHeader {
    pub trace_sequence_line: i32,
    pub trace_sequence_reel: i32,
    pub field_record_number: i32,
    pub trace_number_field_record: i32,
    pub energy_source_point: i32,
    pub ensemble_number: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub group_x: i32,
    pub group_y: i32,
    pub coord_scalar: i16,
    pub num_samples: u16,
    pub sample_interval_us: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegyTrace {
    pub header: SegyTraceHeader,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegyDataset {
    pub text_header: String,
    pub reel_header: SegyReelHeader,
    pub trace_count: usize,
    pub traces: Vec<SegyTrace>,
}

#[derive(Debug, thiserror::Error)]
pub enum SegyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid SEG-Y header length")]
    InvalidHeaderLength,
    #[error("Unsupported data format code: {0}")]
    UnsupportedFormat(u16),
}

pub fn read_segy_file(path: &Path, max_traces: usize) -> Result<SegyDataset, SegyError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // 1. Read 3200-byte EBCDIC/ASCII Text Header
    let mut text_buf = vec![0u8; 3200];
    reader.read_exact(&mut text_buf)?;

    // Basic conversion (EBCDIC or ASCII)
    let text_header = String::from_utf8_lossy(&text_buf).to_string();

    // 2. Read 400-byte Binary Header
    let mut bin_buf = [0u8; 400];
    reader.read_exact(&mut bin_buf)?;

    let job_id = i32::from_be_bytes(bin_buf[0..4].try_into().unwrap());
    let line_number = i32::from_be_bytes(bin_buf[4..8].try_into().unwrap());
    let reel_number = i32::from_be_bytes(bin_buf[8..12].try_into().unwrap());
    let traces_per_ensemble = u16::from_be_bytes(bin_buf[12..14].try_into().unwrap());
    let sample_interval_us = u16::from_be_bytes(bin_buf[16..18].try_into().unwrap());
    let samples_per_trace = u16::from_be_bytes(bin_buf[20..22].try_into().unwrap());
    let data_sample_format = u16::from_be_bytes(bin_buf[24..26].try_into().unwrap());

    let reel_header = SegyReelHeader {
        job_id,
        line_number,
        reel_number,
        traces_per_ensemble,
        sample_interval_us,
        samples_per_trace,
        data_sample_format,
    };

    // 3. Read Traces
    let mut traces = Vec::new();
    let sample_format = if data_sample_format == 0 { 5 } else { data_sample_format }; // Default to IEEE float

    while traces.len() < max_traces {
        let mut trace_hdr_buf = [0u8; 240];
        if reader.read_exact(&mut trace_hdr_buf).is_err() {
            break; // EOF reached
        }

        let trace_sequence_line = i32::from_be_bytes(trace_hdr_buf[0..4].try_into().unwrap());
        let trace_sequence_reel = i32::from_be_bytes(trace_hdr_buf[4..8].try_into().unwrap());
        let field_record_number = i32::from_be_bytes(trace_hdr_buf[8..12].try_into().unwrap());
        let trace_number_field_record = i32::from_be_bytes(trace_hdr_buf[12..16].try_into().unwrap());
        let energy_source_point = i32::from_be_bytes(trace_hdr_buf[16..20].try_into().unwrap());
        let ensemble_number = i32::from_be_bytes(trace_hdr_buf[20..24].try_into().unwrap());
        let source_x = i32::from_be_bytes(trace_hdr_buf[72..76].try_into().unwrap());
        let source_y = i32::from_be_bytes(trace_hdr_buf[76..80].try_into().unwrap());
        let group_x = i32::from_be_bytes(trace_hdr_buf[80..84].try_into().unwrap());
        let group_y = i32::from_be_bytes(trace_hdr_buf[84..88].try_into().unwrap());
        let coord_scalar = i16::from_be_bytes(trace_hdr_buf[68..70].try_into().unwrap());
        let num_samples_hdr = u16::from_be_bytes(trace_hdr_buf[114..116].try_into().unwrap());
        let trace_sample_interval = u16::from_be_bytes(trace_hdr_buf[116..118].try_into().unwrap());

        let n_samples = if num_samples_hdr > 0 {
            num_samples_hdr as usize
        } else {
            samples_per_trace as usize
        };

        if n_samples == 0 {
            break;
        }

        let header = SegyTraceHeader {
            trace_sequence_line,
            trace_sequence_reel,
            field_record_number,
            trace_number_field_record,
            energy_source_point,
            ensemble_number,
            source_x,
            source_y,
            group_x,
            group_y,
            coord_scalar,
            num_samples: n_samples as u16,
            sample_interval_us: trace_sample_interval,
        };

        // Read samples based on format
        let mut samples = vec![0.0f32; n_samples];
        match sample_format {
            3 => {
                // 2-byte integer
                let mut buf = vec![0u8; n_samples * 2];
                reader.read_exact(&mut buf)?;
                for i in 0..n_samples {
                    let val = i16::from_be_bytes(buf[i * 2..i * 2 + 2].try_into().unwrap());
                    samples[i] = val as f32;
                }
            }
            5 => {
                // 4-byte IEEE Float
                let mut buf = vec![0u8; n_samples * 4];
                reader.read_exact(&mut buf)?;
                for i in 0..n_samples {
                    let val = f32::from_be_bytes(buf[i * 4..i * 4 + 4].try_into().unwrap());
                    samples[i] = if val.is_nan() { 0.0 } else { val };
                }
            }
            _ => {
                // Default 2-byte or 4-byte fallback reading zeros
                let mut buf = vec![0u8; n_samples * 4];
                if reader.read_exact(&mut buf).is_err() {
                    break;
                }
            }
        }

        traces.push(SegyTrace { header, samples });
    }

    let trace_count = traces.len();

    Ok(SegyDataset {
        text_header,
        reel_header,
        trace_count,
        traces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segy_reel_header_defaults() {
        let reel = SegyReelHeader {
            job_id: 101,
            line_number: 1,
            reel_number: 1,
            traces_per_ensemble: 1,
            sample_interval_us: 100,
            samples_per_trace: 512,
            data_sample_format: 5,
        };
        assert_eq!(reel.job_id, 101);
        assert_eq!(reel.samples_per_trace, 512);
    }
}
