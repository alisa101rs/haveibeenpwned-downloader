use crate::client::HashPrefix;

// const ALPHABET: [u8; 2] = [
//     b'0', b'1',
// ];
const ALPHABET: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F',
];

pub fn all_ranges_iter() -> impl Iterator<Item = HashPrefix> {
    ALPHABET.iter().flat_map(move |&a| {
        ALPHABET.iter().flat_map(move |&b| {
            ALPHABET.iter().flat_map(move |&c| {
                ALPHABET
                    .iter()
                    .flat_map(move |&d| ALPHABET.iter().map(move |&e| [a, b, c, d, e]))
            })
        })
    })
}

pub const fn total_len() -> u64 {
    (ALPHABET.len() * ALPHABET.len() * ALPHABET.len() * ALPHABET.len() * ALPHABET.len()) as u64
}
