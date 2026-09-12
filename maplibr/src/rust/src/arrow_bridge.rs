use polars::prelude::*;
use polars_arrow::ffi::{export_iterator, ArrowArrayStream, ArrowArrayStreamReader};
use savvy::{savvy, ExternalPointerSexp, Sexp};

/// Build a small hardcoded i32 Series in Rust and stream it out to R
/// through the Arrow C Stream Interface.
///
/// @param stream_ptr An external pointer from `nanoarrow::nanoarrow_allocate_array_stream()`.
/// @export
#[savvy]
fn export_test_series(stream_ptr: Sexp) -> savvy::Result<()> {
    let series = Series::new("x".into(), &[1i32, 2, 3, 4, 5]);

    let field = series.field().to_arrow(CompatLevel::newest());
    let chunks = series.chunks().clone();
    let iter = Box::new(chunks.into_iter().map(Ok));
    let stream = export_iterator(iter, field);

    let stream_ptr = unsafe {
        ExternalPointerSexp::try_from(stream_ptr)?.cast_mut_unchecked::<ArrowArrayStream>()
    };
    unsafe { std::ptr::replace(stream_ptr, stream) };

    Ok(())
}

/// Read an Arrow C stream from R back into a Rust Series, then hand back
/// its length and first value as a sanity check (i32 only, for this prototype).
///
/// @param stream_ptr An external pointer holding a filled ArrowArrayStream.
/// @returns An integer vector: `c(length, first_value)`.
/// @export
#[savvy]
fn import_test_series(stream_ptr: Sexp) -> savvy::Result<Sexp> {
    let mut stream = unsafe {
        let boxed = Box::new(std::ptr::replace(
            ExternalPointerSexp::try_from(stream_ptr)?.cast_mut_unchecked::<ArrowArrayStream>(),
            ArrowArrayStream::empty(),
        ));
        ArrowArrayStreamReader::try_new(boxed).map_err(|e| savvy::Error::new(&e.to_string()))?
    };

    let mut arrays = Vec::new();
    while let Some(arr) = unsafe { stream.next() } {
        arrays.push(arr.map_err(|e| savvy::Error::new(&e.to_string()))?);
    }

    let series = Series::from_arrow_chunks("x".into(), arrays)
        .map_err(|e| savvy::Error::new(&e.to_string()))?;

    let mut out = savvy::OwnedIntegerSexp::new(2)?;
    out[0] = series.len() as i32;
    out[1] = series
        .i32()
        .map_err(|e| savvy::Error::new(&e.to_string()))?
        .get(0)
        .unwrap_or(0);
    Ok(out.into())
}
