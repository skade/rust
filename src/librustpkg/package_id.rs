// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use version::{try_getting_version, try_getting_local_version,
              Version, NoVersion, split_version};
use std::rt::io::Writer;
use std::hash::Streaming;
use std::hash;

/// Path-fragment identifier of a package such as
/// 'github.com/graydon/test'; path must be a relative
/// path with >=1 component.
#[deriving(Clone)]
pub struct PkgId {
    /// This is a path, on the local filesystem, referring to where the
    /// files for this package live. For example:
    /// github.com/mozilla/quux-whatever (it's assumed that if we're
    /// working with a package ID of this form, rustpkg has already cloned
    /// the sources into a local directory in the RUST_PATH).
    path: Path,
    /// Short name. This is the path's filestem, but we store it
    /// redundantly so as to not call get() everywhere (filestem() returns an
    /// option)
    /// The short name does not need to be a valid Rust identifier.
    /// Users can write: `extern mod foo = "...";` to get around the issue
    /// of package IDs whose short names aren't valid Rust identifiers.
    short_name: ~str,
    /// The requested package version.
    version: Version
}

impl Eq for PkgId {
    fn eq(&self, other: &PkgId) -> bool {
        self.path == other.path && self.version == other.version
    }
}

impl PkgId {
    pub fn new(s: &str) -> PkgId {
        use conditions::bad_pkg_id::cond;

        let mut given_version = None;

        // Did the user request a specific version?
        let s = match split_version(s) {
            Some((path, v)) => {
                given_version = Some(v);
                path
            }
            None => {
                s
            }
        };

        let path = Path(s);
        if path.is_absolute {
            return cond.raise((path, ~"absolute pkgid"));
        }
        if path.components.len() < 1 {
            return cond.raise((path, ~"0-length pkgid"));
        }
        let short_name = path.filestem().expect(format!("Strange path! {}", s));

        let version = match given_version {
            Some(v) => v,
            None => match try_getting_local_version(&path) {
                Some(v) => v,
                None => match try_getting_version(&path) {
                    Some(v) => v,
                    None => NoVersion
                }
            }
        };

        PkgId {
            path: path.clone(),
            short_name: short_name.to_owned(),
            version: version
        }
    }

    pub fn hash(&self) -> ~str {
        format!("{}-{}-{}", self.path.to_str(),
                hash(self.path.to_str() + self.version.to_str()),
                self.version.to_str())
    }

    pub fn short_name_with_version(&self) -> ~str {
        format!("{}{}", self.short_name, self.version.to_str())
    }

    /// True if the ID has multiple components
    pub fn is_complex(&self) -> bool {
        self.short_name != self.path.to_str()
    }

    pub fn prefixes_iter(&self) -> Prefixes {
        Prefixes {
            components: self.path.components().to_owned(),
            remaining: ~[]
        }
    }

    // This is the workcache function name for the *installed*
    // binaries for this package (as opposed to the built ones,
    // which are per-crate).
    pub fn install_tag(&self) -> ~str {
        format!("install({})", self.to_str())
    }
}

struct Prefixes {
    priv components: ~[~str],
    priv remaining: ~[~str]
}

impl Iterator<(Path, Path)> for Prefixes {
    #[inline]
    fn next(&mut self) -> Option<(Path, Path)> {
        if self.components.len() <= 1 {
            None
        }
        else {
            let last = self.components.pop();
            self.remaining.push(last);
            // converting to str and then back is a little unfortunate
            Some((Path(self.components.to_str()), Path(self.remaining.to_str())))
        }
    }
}

impl ToStr for PkgId {
    fn to_str(&self) -> ~str {
        // should probably use the filestem and not the whole path
        format!("{}-{}", self.path.to_str(), self.version.to_str())
    }
}


pub fn write<W: Writer>(writer: &mut W, string: &str) {
    writer.write(string.as_bytes());
}

pub fn hash(data: ~str) -> ~str {
    let hasher = &mut hash::default_state();
    write(hasher, data);
    hasher.result_str()
}

