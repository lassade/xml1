# xml1

This create provide a sane, non compliant xml parser without allocations. I just can't describe it better than this.

`xml` is a great format and is expecially good for describing interfaces, that are no more than
things inside other things with some attributes, but it has a few too many crazy features
that bloat everything, `CDATA`, `namespaces` to name a few.

By the way don't worry about the `unsafe`'s inside here it was mostly copied over from `std`.

# Fork it!

I tried to keep the design the simple as possible, you should be able fork it and add any features that you want.

# Milestones

- [x] Parse a "`xml`"
- [ ] Better error handling with `codespan-reporting`
