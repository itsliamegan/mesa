use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::ops::{Index, Range};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Sources {
	sources: Vec<Source>,
	files: Vec<PathBuf>,
	dirs: BTreeSet<PathBuf>,
}

impl Sources {
	pub fn new() -> Self {
		Self {
			sources: Vec::new(),
			files: Vec::new(),
			dirs: BTreeSet::new(),
		}
	}

	pub fn ids(&self) -> impl ExactSizeIterator<Item = SourceId> {
		(0..self.sources.len()).map(SourceId::from_index)
	}

	pub fn files(&self) -> impl ExactSizeIterator<Item = &PathBuf> {
		self.files.iter()
	}

	pub fn dirs(&self) -> impl ExactSizeIterator<Item = &PathBuf> {
		self.dirs.iter()
	}

	pub fn get(&self, id: SourceId) -> &Source {
		&self.sources[id.index()]
	}

	pub fn add(&mut self, file: PathBuf, text: String) -> SourceId {
		let id = SourceId::from_index(self.sources.len());
		self.sources.push(Source {
			id,
			file: file.clone(),
			text,
		});
		self.files.push(file.clone());
		self.dirs.insert(file.parent().unwrap().to_path_buf());
		id
	}

	pub fn loc(&self, span: Span) -> Location {
		self.get(span.src).loc(span.start)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct SourceId(u32);

impl SourceId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Source {
	id: SourceId,
	file: PathBuf,
	text: String,
}

impl Source {
	pub fn id(&self) -> SourceId {
		self.id
	}

	pub fn file(&self) -> &Path {
		&self.file
	}

	pub fn len(&self) -> usize {
		self.text.len()
	}

	pub fn loc(&self, pos: usize) -> Location {
		let mut i = 0;
		let mut lin = 1;
		let mut col = 1;
		while i < pos && i < self.text.len() {
			if self.text.as_bytes()[i] == b'\n' {
				lin += 1;
				col = 1;
			} else {
				col += 1;
			}
			i += 1;
		}
		Location {
			file: self.file.clone(),
			pos: Some((lin, col)),
		}
	}
}

impl Index<usize> for Source {
	type Output = u8;

	fn index(&self, idx: usize) -> &Self::Output {
		&self.text.as_bytes()[idx]
	}
}

impl Index<Range<usize>> for Source {
	type Output = str;

	fn index(&self, idx: Range<usize>) -> &Self::Output {
		&self.text[idx]
	}
}

#[derive(Debug)]
pub struct Location {
	file: PathBuf,
	pos: Option<(usize, usize)>,
}

impl Location {
	pub fn file(file: PathBuf) -> Self {
		Self { file, pos: None }
	}
}

impl Display for Location {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self.pos {
			Some((lin, col)) => write!(f, "{}:{},{}", self.file.to_string_lossy(), lin, col),
			None => write!(f, "{}", self.file.to_string_lossy()),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
	pub src: SourceId,
	pub start: usize,
	pub end: usize,
}
