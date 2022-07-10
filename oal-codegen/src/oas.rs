use openapiv3::ReferenceOr;

/// Converts a [`ReferenceOr<T>`] into a [`ReferenceOr<Box<T>>`].
pub fn into_box_ref<T>(r: ReferenceOr<T>) -> ReferenceOr<Box<T>> {
    match r {
        ReferenceOr::Item(item) => ReferenceOr::boxed_item(item),
        ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
    }
}
