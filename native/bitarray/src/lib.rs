
use rustler::{Atom, Env, NifResult, ResourceArc,Binary,OwnedBinary,Encoder, Term};
use std::sync::Mutex;
use std::cmp;
mod atoms {
    rustler::atoms! {
        ok,
        eof
    }
}
struct BitArray {
    pub data: Mutex<Box<[u64]>>,
}
const CHUNK_SIZE_U64: usize = 1024;

#[rustler::nif]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[rustler::nif]
fn sub(a: i64, b: i64) -> i64 {
    a - b
}

// Implement the Resource trait for BitArray
// impl rustler::Resource for BitArray {}

#[rustler::nif]
fn new(length: usize) -> NifResult<ResourceArc<BitArray>> {
    let data: Box<[u64]> = vec![0; (length + 63) / 64].into_boxed_slice();
    // println!("{:?}", data);
    let resource: ResourceArc<BitArray> = ResourceArc::new(BitArray {
        data: Mutex::new(data),
    });
    Ok(resource)
}

#[rustler::nif]
fn put(resource: ResourceArc<BitArray>, index: usize, value: bool) -> Atom {
    let mut vec = resource.data.lock().unwrap();
    let mut word = vec[index / 64];

    if value {
        word |= 1 << (index % 64);
    } else {
        word &= !(1 << (index % 64));
    }

    vec[index / 64] = word;

    atoms::ok()
}

#[rustler::nif]
fn to_bin_chunked(env: Env, resource: ResourceArc<BitArray>, chunk_num: usize) -> NifResult<(Term, Binary)> {
    let data = resource.data.lock().unwrap();
    let offset = chunk_num * CHUNK_SIZE_U64;
    let reminding = (data.len() as isize) - (offset as isize);
    let size = std::cmp::min(CHUNK_SIZE_U64 as isize, reminding) as usize;
    let is_eof = reminding <= (CHUNK_SIZE_U64 as isize);

    let erl_bin_size = size * 8;
    let mut erl_bin = OwnedBinary::new(erl_bin_size).ok_or_else(|| rustler::Error::Term(Box::new("Binary alloc failed")))?;
    let bin = erl_bin.as_mut_slice();

    for x in 0..size {
        for y in 0..8 {
            let i = x * 8 + y;
            bin[i] = (data[x + offset] >> (y * 8)) as u8;
        }
    }
    if is_eof {
        Ok((atoms::eof().encode(env), erl_bin.release(env)))
    } else {
        Ok(((chunk_num + 1).encode(env), erl_bin.release(env)))
    }
  
}


#[rustler::nif]
fn or_chunk(resource: ResourceArc<BitArray>, bin: Binary, byte_offset: usize) -> NifResult<usize> {
    let mut data = resource.data.lock().unwrap();

    for x in 0..bin.len() {
        let data_index = (x + byte_offset) / 8;
        let bin_offset = (x + byte_offset) % 8;

        data[data_index] |= (bin[x] as u64) << (bin_offset * 8);
    }

    Ok(byte_offset + bin.len())
}

#[rustler::nif]
fn count_ones(resource: ResourceArc<BitArray>) -> usize {
    let data = resource.data.lock().unwrap();
    data.iter().map(|x| x.count_ones() as usize).sum()
}

#[rustler::nif]
fn get(resource: ResourceArc<BitArray>, index: usize) -> bool {
    let data = resource.data.lock().unwrap();
    (data[index / 64] & (1 << (index % 64))) != 0
}

#[rustler::nif]
fn bit_length(resource: ResourceArc<BitArray>) -> usize {
    let data = resource.data.lock().unwrap();
    data.len() * 64
}

#[rustler::nif]
fn count_ones_chunked(env: Env, resource: ResourceArc<BitArray>, chunk_num: usize) -> NifResult<(Term)> {
    let data = resource.data.lock().unwrap();

    let offset = chunk_num * CHUNK_SIZE_U64;
    let remaining = data.len().saturating_sub(offset);
    let size = cmp::min(CHUNK_SIZE_U64, remaining);
    let is_eof = remaining <= CHUNK_SIZE_U64;

    let mut count = 0usize;

    for x in 0..size {
        count += data[x + offset].count_ones() as usize;
    }

    // let env = unsafe { rustler::Env::new() };
    if is_eof {
        Ok((atoms::eof(), count).encode(env))
    }else {
        Ok((chunk_num + 1, count).encode(env))
    }
}
fn on_load(env: Env, _term: Term) -> bool {
    rustler::resource!(BitArray, env);
    true
}

rustler::init!("Elixir.Flower.Native.BitArray", load = on_load);
