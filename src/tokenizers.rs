use std::ffi::CStr;
use std::path::PathBuf;
use std::ptr;
use tokenizers::tokenizer::Tokenizer;

/// Configuration options for tokenizer initialization.
///
/// Controls how the tokenizer handles special tokens during encoding.
#[repr(C)]
pub struct TokenizerOptions {
    encode_special_tokens: bool,
}

/// C-compatible buffer containing tokenization results.
///
/// This structure holds the output from tokenization operations, including
/// token IDs and various metadata. All pointer fields must be freed with
/// `free_buffer` to avoid memory leaks.
#[repr(C)]
pub struct Buffer {
    ids: *mut u32,
    type_ids: *mut u32,
    special_tokens_mask: *mut u32,
    attention_mask: *mut u32,
    tokens: *mut *mut libc::c_char,
    offsets: *mut usize,
    len: usize,
}

/// Creates a tokenizer from a byte array.
///
/// Loads a tokenizer from serialized bytes (typically a JSON configuration).
///
/// # Arguments
///
/// * `bytes` - Pointer to the byte array containing the tokenizer configuration
/// * `len` - Length of the byte array
/// * `opts` - Tokenizer options controlling special token handling
///
/// # Returns
///
/// Pointer to the created `Tokenizer` instance. Must be freed with `free_tokenizer`.
///
/// # Safety
///
/// - `bytes` must point to valid memory of at least `len` bytes
/// - The returned pointer must be freed with `free_tokenizer`
/// - Panics if the bytes don't contain a valid tokenizer configuration
#[allow(clippy::missing_panics_doc, clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn from_bytes(
    bytes: *const u8,
    len: u32,
    opts: &TokenizerOptions,
) -> *mut Tokenizer {
    let bytes_slice = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    let mut tokenizer = Tokenizer::from_bytes(bytes_slice).expect("failed to create tokenizer");
    tokenizer.set_encode_special_tokens(opts.encode_special_tokens);
    Box::into_raw(Box::new(tokenizer))
}

/// TODO merge with `from_bytes` and pass truncation params as an argument to `TokenizerOptions`
#[allow(clippy::missing_panics_doc, clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn from_bytes_with_truncation(
    bytes: *const u8,
    len: u32,
    max_len: usize,
    dir: u8,
) -> *mut Tokenizer {
    let bytes_slice = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    let tokenizer: Tokenizer = Tokenizer::from_bytes(bytes_slice)
        .expect("failed to create tokenizer")
        .with_truncation(Some(tokenizers::tokenizer::TruncationParams {
            max_length: max_len,
            direction: match dir {
                0 => tokenizers::tokenizer::TruncationDirection::Left,
                1 => tokenizers::tokenizer::TruncationDirection::Right,
                _ => panic!("invalid truncation direction"),
            },
            ..Default::default()
        }))
        .unwrap()
        .to_owned()
        .into();
    Box::into_raw(Box::new(tokenizer))
}

/// Creates a tokenizer from a file path.
///
/// Loads a tokenizer from a JSON configuration file.
///
/// # Arguments
///
/// * `config` - Null-terminated C string containing the file path
///
/// # Returns
///
/// Pointer to the created `Tokenizer` instance, or null if loading fails.
/// Must be freed with `free_tokenizer`.
///
/// # Safety
///
/// - `config` must be a valid null-terminated C string
/// - The returned pointer must be freed with `free_tokenizer`
#[allow(clippy::missing_panics_doc, clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn from_file(config: *const libc::c_char) -> *mut libc::c_void {
    let config_cstr = unsafe { CStr::from_ptr(config) };
    let config = config_cstr.to_str().unwrap();
    let config = PathBuf::from(config);
    match Tokenizer::from_file(config) {
        Ok(tokenizer) => {
            let ptr = Box::into_raw(Box::new(tokenizer));
            ptr.cast()
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Options controlling what information is returned from encoding.
///
/// Determines which fields in the returned `Buffer` will be populated.
#[repr(C)]
pub struct EncodeOptions {
    add_special_tokens: bool,

    return_type_ids: bool,
    return_tokens: bool,
    return_special_tokens_mask: bool,
    return_attention_mask: bool,
    return_offsets: bool,
}

/// Encodes a text string into token IDs.
///
/// Tokenizes the input text and returns a `Buffer` containing the results.
/// The buffer contents depend on the `EncodeOptions` configuration.
///
/// # Arguments
///
/// * `ptr` - Pointer to a `Tokenizer` instance
/// * `message` - Null-terminated C string to encode
/// * `options` - Options controlling what information to return
///
/// # Returns
///
/// A `Buffer` containing the encoding results. Must be freed with `free_buffer`.
///
/// # Safety
///
/// - `ptr` must be a valid `Tokenizer` pointer
/// - `message` must be a valid null-terminated C string
/// - The returned `Buffer` must be freed with `free_buffer`
#[allow(clippy::missing_panics_doc, clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode(
    ptr: *mut libc::c_void,
    message: *const libc::c_char,
    options: &EncodeOptions,
) -> Buffer {
    let tokenizer: &Tokenizer;
    unsafe {
        tokenizer = ptr
            .cast::<Tokenizer>()
            .as_ref()
            .expect("failed to cast tokenizer");
    }
    let message_cstr = unsafe { CStr::from_ptr(message) };
    let message = message_cstr.to_str();
    if message.is_err() {
        return Buffer {
            ids: ptr::null_mut(),
            tokens: ptr::null_mut(),
            len: 0,
            type_ids: ptr::null_mut(),
            special_tokens_mask: ptr::null_mut(),
            attention_mask: ptr::null_mut(),
            offsets: ptr::null_mut(),
        };
    }

    let encoding = tokenizer
        .encode(message.unwrap(), options.add_special_tokens)
        .expect("failed to encode input");
    let mut vec_ids = encoding.get_ids().to_vec();
    vec_ids.shrink_to_fit();
    let ids = vec_ids.as_mut_ptr();
    let len = vec_ids.len();
    std::mem::forget(vec_ids);

    let mut type_ids: *mut u32 = ptr::null_mut();
    if options.return_type_ids {
        let mut vec_type_ids = encoding.get_type_ids().to_vec();
        vec_type_ids.shrink_to_fit();
        type_ids = vec_type_ids.as_mut_ptr();
        std::mem::forget(vec_type_ids);
    }

    let mut tokens: *mut *mut libc::c_char = ptr::null_mut();
    if options.return_tokens {
        let mut vec_tokens = encoding
            .get_tokens()
            .to_vec()
            .into_iter()
            .map(|s| std::ffi::CString::new(s).unwrap().into_raw())
            .collect::<Vec<_>>();
        vec_tokens.shrink_to_fit();
        tokens = vec_tokens.as_mut_ptr();
        std::mem::forget(vec_tokens);
    }

    let mut special_tokens_mask: *mut u32 = ptr::null_mut();
    if options.return_special_tokens_mask {
        let mut vec_special_tokens_mask = encoding.get_special_tokens_mask().to_vec();
        vec_special_tokens_mask.shrink_to_fit();
        special_tokens_mask = vec_special_tokens_mask.as_mut_ptr();
        std::mem::forget(vec_special_tokens_mask);
    }

    let mut attention_mask: *mut u32 = ptr::null_mut();
    if options.return_attention_mask {
        let mut vec_attention_mask = encoding.get_attention_mask().to_vec();
        vec_attention_mask.shrink_to_fit();
        attention_mask = vec_attention_mask.as_mut_ptr();
        std::mem::forget(vec_attention_mask);
    }

    let mut offsets: *mut usize = ptr::null_mut();
    if options.return_offsets {
        let vec_offsets_tuples = encoding.get_offsets().to_vec();
        let mut vec_offsets = Vec::with_capacity(vec_offsets_tuples.len() * 2);
        for i in vec_offsets_tuples {
            vec_offsets.push(i.0);
            vec_offsets.push(i.1);
        }
        vec_offsets.shrink_to_fit();
        offsets = vec_offsets.as_mut_ptr();
        std::mem::forget(vec_offsets);
    }

    Buffer {
        ids,
        type_ids,
        special_tokens_mask,
        attention_mask,
        tokens,
        offsets,
        len,
    }
}

/// Decodes token IDs back into text.
///
/// Converts an array of token IDs back into a human-readable string.
///
/// # Arguments
///
/// * `ptr` - Pointer to a `Tokenizer` instance
/// * `ids` - Array of token IDs to decode
/// * `len` - Number of token IDs in the array
/// * `skip_special_tokens` - Whether to omit special tokens from the output
///
/// # Returns
///
/// Null-terminated C string containing the decoded text, or null if decoding fails.
/// Must be freed with `free_string`.
///
/// # Safety
///
/// - `ptr` must be a valid `Tokenizer` pointer
/// - `ids` must point to an array of at least `len` u32 values
/// - The returned string must be freed with `free_string`
#[allow(clippy::missing_panics_doc, clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode(
    ptr: *mut libc::c_void,
    ids: *const u32,
    len: u32,
    skip_special_tokens: bool,
) -> *mut libc::c_char {
    let tokenizer: &Tokenizer;
    unsafe {
        tokenizer = ptr
            .cast::<Tokenizer>()
            .as_ref()
            .expect("failed to cast tokenizer");
    }
    let ids_slice = unsafe { std::slice::from_raw_parts(ids, len as usize) };

    let string = tokenizer
        .decode(ids_slice, skip_special_tokens)
        .expect("failed to decode input");
    match std::ffi::CString::new(string) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Returns the vocabulary size of the tokenizer.
///
/// Gets the total number of tokens in the tokenizer's vocabulary,
/// including special tokens.
///
/// # Arguments
///
/// * `ptr` - Pointer to a `Tokenizer` instance
///
/// # Returns
///
/// The vocabulary size as a u32
///
/// # Safety
///
/// - `ptr` must be a valid `Tokenizer` pointer
#[allow(clippy::missing_panics_doc, clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vocab_size(ptr: *mut libc::c_void) -> u32 {
    let tokenizer: &Tokenizer;
    unsafe {
        tokenizer = ptr
            .cast::<Tokenizer>()
            .as_ref()
            .expect("failed to cast tokenizer");
    }
    u32::try_from(tokenizer.get_vocab_size(true)).unwrap()
}

/// Frees a tokenizer instance.
///
/// Deallocates memory for a tokenizer created by `from_bytes` or `from_file`.
///
/// # Arguments
///
/// * `ptr` - Pointer to a `Tokenizer` instance to free
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned from `from_bytes` or `from_file`
/// - `ptr` must not be used after calling this function
/// - Calling with a null pointer is safe (no-op)
#[unsafe(no_mangle)]
pub extern "C" fn free_tokenizer(ptr: *mut ::libc::c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr.cast::<Tokenizer>()));
    }
}

/// Frees a buffer returned from encoding.
///
/// Deallocates all memory associated with a `Buffer` structure,
/// including all internal arrays.
///
/// # Arguments
///
/// * `buf` - The `Buffer` to free
///
/// # Safety
///
/// - `buf` must be a valid `Buffer` returned from `encode`
/// - `buf` must not be used after calling this function
/// - All pointers in the buffer must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn free_buffer(buf: Buffer) {
    if !buf.ids.is_null() {
        unsafe {
            Vec::from_raw_parts(buf.ids, buf.len, buf.len);
        }
    }
    if !buf.type_ids.is_null() {
        unsafe {
            Vec::from_raw_parts(buf.type_ids, buf.len, buf.len);
        }
    }
    if !buf.special_tokens_mask.is_null() {
        unsafe {
            Vec::from_raw_parts(buf.special_tokens_mask, buf.len, buf.len);
        }
    }
    if !buf.attention_mask.is_null() {
        unsafe {
            Vec::from_raw_parts(buf.attention_mask, buf.len, buf.len);
        }
    }
    if !buf.offsets.is_null() {
        unsafe {
            Vec::from_raw_parts(buf.offsets, buf.len * 2, buf.len * 2);
        }
    }
    if !buf.tokens.is_null() {
        unsafe {
            let strings = Vec::from_raw_parts(buf.tokens, buf.len, buf.len);
            for s in strings {
                drop(std::ffi::CString::from_raw(s.cast::<libc::c_char>()));
            }
        }
    }
}

/// Frees a C string returned from decode.
///
/// Deallocates memory for a string returned by `decode`.
///
/// # Arguments
///
/// * `ptr` - Pointer to a C string to free
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned from `decode`
/// - `ptr` must not be used after calling this function
/// - Calling with a null pointer is safe (no-op)
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_string(ptr: *mut libc::c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(std::ffi::CString::from_raw(ptr));
    }
}
