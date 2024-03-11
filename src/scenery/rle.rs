use std::fmt::Display;

use thiserror::Error;

/// Run Length Encoding
pub fn rle_encode(data: &[u16]) -> Vec<i16> {
    let mut encoding: Vec<i16> = Vec::new();
    let mut run_start = 0;
    let mut i = 2;

    while i < data.len() {
        // Look for three in a row that are the same.
        let n = data[i];
        if n == data[i - 1] && n == data[i - 2] {
            // Encode literal
            let mut run_length = i - 2 - run_start;
            if run_length > 0 {
                encoding.push((run_length * 2) as i16);
                while run_start < i - 2 {
                    encoding.push(data[run_start] as i16);
                    run_start += 1;
                }
            }

            // Encode repeat
            run_start = i - 2;
            i += 1;
            while i < data.len() && data[i] == n {
                i += 1;
            }
            run_length = i - run_start;
            encoding.push((run_length * 2 + 1) as i16);
            encoding.push(n as i16);
            run_start = i;
        } else {
            i += 1;
        }
    }

    // Flush last literal
    if run_start < data.len() {
        let run_length = data.len() - run_start;
        if run_length > 0 {
            encoding.push((run_length * 2) as i16);
            while run_start < data.len() {
                encoding.push(data[run_start] as i16);
                run_start += 1;
            }
        }
    }

    encoding
}

/// Run Length Decoding
pub fn rle_decode<'a>(
    data: &[i16],
    uncompressed: &'a mut [u16],
) -> Result<&'a [u16], RLEDecodeError> {
    let mut j = 0;
    let iter = &mut data.iter();
    while let Some(n) = iter.next() {
        let len = (n / 2) as usize;
        if j + len > uncompressed.len() {
            return Err(RLEDecodeError);
        }
        if n & 1 != 0 {
            // Repeat
            match iter.next() {
                Some(v) => {
                    let v = (v & 0x7fff) as u16;
                    for _ in 0..len {
                        uncompressed[j] = v;
                        j += 1;
                    }
                }
                None => return Err(RLEDecodeError),
            }
        } else {
            // Literal
            for _ in 0..len {
                match iter.next() {
                    Some(v) => {
                        uncompressed[j] = (v & 0x7fff) as u16;
                        j += 1;
                    }
                    None => return Err(RLEDecodeError),
                }
            }
        }
    }

    if j != uncompressed.len() {
        return Err(RLEDecodeError);
    }

    Ok(uncompressed)
}

#[derive(Debug, Error)]
pub struct RLEDecodeError;

impl Display for RLEDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RLEDecodeError")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(rle_encode(&[0, 0, 0, 0]), [9, 0]);
        assert_eq!(rle_encode(&[1, 2, 3, 4]), [8, 1, 2, 3, 4]);
        assert_eq!(rle_encode(&[0, 0, 0, 0, 1, 1, 1, 1]), [9, 0, 9, 1]);
        assert_eq!(rle_encode(&[1, 2, 3, 4, 0, 0, 0, 0]), [8, 1, 2, 3, 4, 9, 0]);
        assert_eq!(rle_encode(&[0, 0, 0, 0, 1, 2, 3, 4]), [9, 0, 8, 1, 2, 3, 4]);
    }

    #[test]
    fn test_decode() -> Result<(), RLEDecodeError> {
        let out: &mut [u16] = &mut [0; 4];
        assert_eq!(rle_decode(&rle_encode(&[0, 0, 0, 0]), out)?, [0, 0, 0, 0]);
        assert_eq!(rle_decode(&rle_encode(&[1, 2, 3, 4]), out)?, [1, 2, 3, 4]);
        let out: &mut [u16] = &mut [0; 8];
        assert_eq!(
            rle_decode(&rle_encode(&[0, 0, 0, 0, 1, 1, 1, 1]), out)?,
            [0, 0, 0, 0, 1, 1, 1, 1]
        );
        assert_eq!(
            rle_decode(&rle_encode(&[1, 2, 3, 4, 0, 0, 0, 0]), out)?,
            [1, 2, 3, 4, 0, 0, 0, 0]
        );
        assert_eq!(
            rle_decode(&rle_encode(&[0, 0, 0, 0, 1, 2, 3, 4]), out)?,
            [0, 0, 0, 0, 1, 2, 3, 4]
        );

        Ok(())
    }
}
