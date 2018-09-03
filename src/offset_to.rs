use std::mem;

pub trait OffsetTo {
    fn ptr_offset_to(self, other: Self) -> Option<usize>;
}

impl<T: Sized> OffsetTo for *const T {
    fn ptr_offset_to(self, other: Self) -> Option<usize> {
        let size = mem::size_of::<T>();
        if size == 0 {
            None
        } else {
            assert!(0 < size && size <= isize::max_value() as usize);
            let d = isize::wrapping_sub(other as _, self as _);
            Some(d.wrapping_div(size as _) as _)
        }
    }
}

impl<T: Sized> OffsetTo for *mut T {
    fn ptr_offset_to(self, other: Self) -> Option<usize> {
        let size = mem::size_of::<T>();
        if size == 0 {
            None
        } else {
            assert!(0 < size && size <= isize::max_value() as usize);
            let d = isize::wrapping_sub(other as _, self as _);
            Some(d.wrapping_div(size as _) as _)
        }
    }
}
