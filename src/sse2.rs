#![allow(unused)]

//use alloc::vec::Vec;
#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
use core::ptr::null;

use crate::{str_from_range, XmlEvent};

struct Parser {
    ptr: *const u8,
    ptr_end: *const u8,
    line_count: usize,
    line_ptr: *const u8,
}

impl Parser {
    fn new() -> Self {
        Self {
            ptr: null(),
            ptr_end: null(),
            line_count: 0,
            line_ptr: null(),
        }
    }

    #[inline(always)]
    unsafe fn mask_and_find(&mut self, f: impl Fn(__m128i) -> i32) -> bool {
        while self.ptr < self.ptr_end {
            let chunk = _mm_loadu_si128(self.ptr as *const _); // 6 cycles
            let mask = (f)(chunk); // 8 cycles
            if mask != 0 {
                // found something
                let offset = mask.trailing_zeros() as usize;

                // out of bounds check
                self.ptr = self.ptr.add(offset);

                return true;
            } else {
                self.ptr = self.ptr.add(16);
            }
        }

        false
    }

    // todo: or enter
    unsafe fn ignore_space(&mut self) -> bool {
        self.mask_and_find(|chunk| {
            !_mm_movemask_epi8(_mm_or_si128(
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8)), // 0x20 (32)
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\t' as i8)), // 0x0B (11)
            )) // 8 cycles
        })
    }

    unsafe fn find(&mut self, ch: u8) -> bool {
        self.mask_and_find(|chunk| {
            _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, _mm_set1_epi8(ch as i8))) // 4 cycles
        })
    }

    unsafe fn find_space(&mut self) -> bool {
        self.mask_and_find(|chunk| {
            _mm_movemask_epi8(_mm_or_si128(
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8)), // 0x20 (32)
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\t' as i8)), // 0x0B (11)
            )) // 8 cycles
        })
    }

    unsafe fn find_space_or_enter(&mut self) -> bool {
        self.mask_and_find(|chunk| {
            _mm_movemask_epi8(_mm_or_si128(
                _mm_or_si128(
                    _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8)), // 0x20 (32)
                    _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\t' as i8)), // 0x0B (11)
                ),
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\n' as i8)), // 0x0A (10)
            )) // 10 cycles
        })
    }

    #[inline(always)]
    fn enter(&mut self) {
        self.line_count += 1;
        self.line_ptr = self.ptr;
    }

    unsafe fn parse<'a>(&mut self, input: &'a str, mut f: impl FnMut(XmlEvent<'a>)) {
        self.ptr = input.as_bytes().as_ptr();
        self.ptr_end = self.ptr.add(input.len());
        self.line_ptr = self.ptr;

        while self.ptr < self.ptr_end {
            if !self.ignore_space() {
                // nothing left
                return;
            }

            let ch = *self.ptr;

            if ch == b'\n' {
                // consume '\n'
                self.ptr = self.ptr.add(1);

                self.enter();

                continue;
            }

            if ch == b'<' {
                // begin element

                // skip '<'
                self.ptr = self.ptr.add(1);

                // read tag
                let tag_ptr = self.ptr;
                if !self.find_space_or_enter() {
                    // empty tag, nothing left todo
                    return;
                }

                let name = str_from_range(tag_ptr, self.ptr);
                (f)(XmlEvent::PushElement { name });

                // handle new line
                if *self.ptr == b'\n' {
                    // consume '\n'
                    self.ptr = self.ptr.add(1);

                    self.enter();
                }

                // todo: read attributes
                while self.ptr < self.ptr_end {
                    if !self.ignore_space() {
                        // nothing left
                        return;
                    }

                    let ch = *self.ptr;

                    if ch == b'/' {
                        // skip '/'
                        self.ptr = self.ptr.add(1);
                        if self.ptr < self.ptr_end {
                            if *self.ptr == b'>' {
                                (f)(XmlEvent::PopElement { name: None });
                                // skip '>'
                                self.ptr = self.ptr.add(1);
                                continue;
                            } else {
                                panic!("expecting `/>`")
                            }
                        }
                    }

                    if ch == b'>' {
                        // done reading attributes

                        // skip '>'
                        self.ptr = self.ptr.add(1);
                        break;
                    }

                    if ch == b'=' || ch == b'\"' {
                        panic!("unexpected char");
                    }

                    // read attribute name
                    let attr_ptr = self.ptr;

                    let mut break_mask = 0;
                    while self.ptr < self.ptr_end {
                        let ch = *self.ptr;
                        if ch == b'=' || ch == b'=' {
                            break;
                        }
                        self.ptr = self.ptr.add(1);
                    }

                    if self.ptr >= self.ptr_end {
                        self.ptr = self.ptr_end;
                        let name = str_from_range(attr_ptr, self.ptr);
                        (f)(XmlEvent::Attr { name, value: None });
                        return;
                    }

                    if break_mask & 0b1_0000 != 0 {}
                }
            } else {
                // push text
            }
        }
    }
}
