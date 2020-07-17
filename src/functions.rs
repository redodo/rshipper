use rand::seq::index::IndexVec;
use rand::Rng;
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use rpassword;
use scrypt::{scrypt, ScryptParams};
use zeroize::Zeroize;

pub fn encode(payload: &Vec<u8>, password: &String, container: &mut Vec<u8>) -> Result<(), String> {
    let head_bytes = 4;
    let head_bits = head_bytes * 8;
    let body_bytes = payload.len();
    let body_bits = body_bytes * 8;
    let payload_bits = head_bits + body_bits;

    let container_size = container.len();
    if container_size < payload_bits {
        return Err(format!(
            "Payload too large for container: {} bits needed, {} bits available",
            payload_bits, container_size
        ));
    }

    println!("Hashing password...");
    // TODO: Manage hash as secret
    let mut hash = hash_password(&password);

    // TODO: Validate payload size < usize::MAX, and container_size < usize::MAX
    println!("Generating random sequence...");
    let mut sequence = create_sequence(hash, payload_bits, container_size);
    let mut sequencer = sequence.iter();

    // TODO: Encrypt payload
    println!("Scrubbing password...");
    hash.zeroize();

    println!("Encoding payload...");
    let length_header = encode_length_header(body_bytes);
    encode_bytes(&length_header.to_vec(), &mut sequencer, container)?;
    encode_bytes(&payload, &mut sequencer, container)?;

    println!("Scrubbing random sequence...");
    sequence.zeroize();

    Ok(())
}
pub fn decode(password: &String, container: &mut Vec<u8>) -> Vec<u8> {
    let head_bytes = 4;
    let head_bits = head_bytes * 8;

    let container_size = container.len();

    println!("Hashing password...");
    // TODO: Manage hash as secret
    let mut hash = hash_password(&password);

    // TODO: Validate payload size < usize::MAX, and container_size < usize::MAX
    // Create a sequencer of length 4 to obtain the payload size
    let hash_copy = hash.clone();
    let len_sequence = create_sequence(hash_copy, head_bits, container_size.clone());
    let mut len_sequencer = len_sequence.iter();
    let length_header = decode_bytes(head_bytes, &mut len_sequencer, &container).unwrap();
    let body_bytes = decode_length_header(length_header);
    let body_bits = body_bytes * 8;

    let sequence = create_sequence(hash, head_bits + body_bits, container_size);
    let mut sequencer = sequence[head_bits..].iter();

    let payload = decode_bytes(body_bytes, &mut sequencer, &container).unwrap();
    hash.zeroize();
    payload
}
pub fn prompt_password(confirm: bool) -> Result<String, String> {
    let password = rpassword::read_password_from_tty(Some("Passphrase: ")).unwrap();
    if confirm {
        let password_confirm =
            rpassword::read_password_from_tty(Some("Confirm passphrase: ")).unwrap();
        if password != password_confirm {
            return Err(format!("Confirmation didn't match passphrase."));
        }
    }
    Ok(password)
}

fn hash_password(password: &String) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let params = ScryptParams::new(12, 8, 1).unwrap();
    let bytes = &password.clone().into_bytes();
    scrypt(&bytes, &[], &params, &mut hash).unwrap();
    hash
}
fn create_sequence(seed: [u8; 32], length: usize, max: usize) -> Vec<usize> {
    let mut rng = ChaChaRng::from_seed(seed);
    sample_inplace(&mut rng, max as u32, length as u32).into_vec()
}
/// Copy of rand::seq::index::sample_inplace, as index::sample will use various
/// different functions based on the input, causing inconsistent results.
fn sample_inplace<R>(rng: &mut R, length: u32, amount: u32) -> IndexVec
where
    R: Rng + ?Sized,
{
    debug_assert!(amount <= length);
    let mut indices: Vec<u32> = Vec::with_capacity(length as usize);
    indices.extend(0..length);
    for i in 0..amount {
        let j: u32 = rng.gen_range(i, length);
        indices.swap(i as usize, j as usize);
    }
    indices.truncate(amount as usize);
    debug_assert_eq!(indices.len(), amount as usize);
    IndexVec::from(indices)
}

fn encode_length_header(length: usize) -> Vec<u8> {
    vec![
        (length >> 24 & 0xFF) as u8,
        (length >> 16 & 0xFF) as u8,
        (length >> 8 & 0xFF) as u8,
        (length & 0xFF) as u8,
    ]
}
fn decode_length_header(header: Vec<u8>) -> usize {
    let mut length = 0;
    for byte in header {
        length <<= 8;
        length |= byte as usize;
    }
    length
}
fn encode_bytes<'a, T: Iterator<Item = &'a usize>>(
    payload: &Vec<u8>,
    sequencer: &mut T,
    container: &mut Vec<u8>,
) -> Result<(), String> {
    for byte in payload {
        for s in 0..8 {
            let bit = byte >> s & 1;
            match sequencer.next() {
                Some(&i) => container[i] = container[i] & 0b11111110 | bit,
                None => return Err(format!("sequence prematurely exhausted")),
            }
        }
    }
    Ok(())
}
fn decode_bytes<'a, T: Iterator<Item = &'a usize>>(
    length: usize,
    sequencer: &mut T,
    container: &Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for _ in 0..length {
        let mut byte = 0u8;
        for s in 0..8 {
            match sequencer.next() {
                Some(&i) => byte |= (container[i] & 1) << s,
                None => return Err(format!("sequence prematurely exhausted")),
            }
        }
        bytes.push(byte);
    }
    Ok(bytes)
}
