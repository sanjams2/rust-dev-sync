use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Debug)]
struct Node<T> {
    data: Option<T>,
    children: HashMap<OsString, Node<T>>,
}

impl<T> Node<T> {
    fn empty() -> Self {
        Node {
            data: None,
            children: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct FsTree<T> {
    root: Node<T>,
}

impl<T> FsTree<T> {
    pub fn new() -> Self {
        FsTree {
            root: Node::empty(),
        }
    }

    pub fn insert(&mut self, path: &PathBuf, data: T) {
        let mut path_vec: Vec<&OsStr> = path.iter().rev().collect();
        if path_vec.first().map_or(false, |p| p.is_empty()) {
            path_vec.remove(0);
        }
        if path_vec.last() != Some(&OsStr::new("/")) {
            panic!("path must be absolute");
        }
        FsTree::insert_with_path(&mut self.root, path_vec, data);
    }

    fn insert_with_path(node: &mut Node<T>, mut path: Vec<&OsStr>, data: T) {
        if path.len() == 0 {
            node.data = Some(data);
            return;
        }
        let cur = path.pop().unwrap();
        if !node.children.contains_key(cur) {
            node.children.insert(OsString::from(cur), Node::empty());
        }
        let child = node.children.get_mut(cur).unwrap();
        FsTree::insert_with_path(child, path, data);
    }

    pub fn get_closest(&self, path: &PathBuf) -> Option<&T> {
        let path_vec: Vec<&OsStr> = path.iter().rev().collect();
        FsTree::get_with_path(&self.root, path_vec)
    }

    fn get_with_path<'a>(node: &'a Node<T>, mut path: Vec<&OsStr>) -> Option<&'a T> {
        path.pop()
            .and_then(|child_name| node.children.get(child_name))
            .and_then(|child| FsTree::get_with_path(child, path))
            .or(node.data.as_ref())
        /*
        if let Some(child_name) = path.pop() {
            if let Some(child) = node.children.get(child_name) {
                let res = FsTree::get_with_path(child, path);
                if res.is_some() {
                    return res;
                }
            }
        }
        node.data.as_ref()
        */
    }
}

// Im not sure if using a iterator is the most optimal thing to do since we have to traverse all
// nodes in the tree and some nodes may not have data.
// It could be better for the FsTree object to  simply have a vector of references to existing
// nodes with data (or the data itself). But that is challenging as it requires either using
// references counters, or unsafe + Box::pin to support self referential structs
// For now, Im using this iterator as an attempt to practice some common rust patterns
impl<'a, T> IntoIterator for &'a FsTree<T> {
    type Item = &'a T;
    type IntoIter = FsTreeIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        FsTreeIter {
            to_check: vec![&self.root],
        }
    }
}

pub struct FsTreeIter<'a, T> {
    to_check: Vec<&'a Node<T>>,
}

impl<'a, T> Iterator for FsTreeIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut data: Option<&T> = None;
        while data.is_none() && !self.to_check.is_empty() {
            let next_check = self.to_check.pop();
            data = next_check.and_then(|node| node.data.as_ref());
            next_check.map(|node| {
                node.children.values().for_each(|child| {
                    self.to_check.push(child);
                });
            });
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::fstree::FsTree;

    #[derive(Debug, PartialEq, Clone)]
    struct Item {
        i: i8,
        s: String,
    }

    #[test]
    fn it_works() {
        let mut tree: FsTree<Item> = FsTree::new();
        let item1 = Item {
            i: 1,
            s: String::from("s1"),
        };
        let item2 = Item {
            i: 2,
            s: String::from("s2"),
        };
        let item3 = Item {
            i: 3,
            s: String::from("s3"),
        };
        tree.insert(
            &PathBuf::from("/tmp/dir1/subdir1/subsubdir1"),
            item1.clone(),
        );
        tree.insert(
            &PathBuf::from("/tmp/dir2/subdir1/subsubdir2/"),
            item2.clone(),
        );
        tree.insert(&PathBuf::from("/tmp/dir1"), item3.clone());
        assert_eq!(
            tree.get_closest(&PathBuf::from("/tmp/dir1/file1")).unwrap(),
            &item3
        );
        assert_eq!(
            tree.get_closest(&PathBuf::from("/tmp/dir1/subdir1/file1"))
                .unwrap(),
            &item3
        );
        assert_eq!(
            tree.get_closest(&PathBuf::from(
                "/tmp/dir1/subdir1/subsubdir1/subsubsubdir1/file1"
            ))
            .unwrap(),
            &item1
        );
        assert_eq!(
            tree.get_closest(&PathBuf::from("/tmp/dir2/subdir1/subsubdir2/file1"))
                .unwrap(),
            &item2
        );
        assert_eq!(tree.get_closest(&PathBuf::from("/random/file1")), None);
        let item4 = Item {
            i: 4,
            s: String::from("s4"),
        };
        tree.insert(&PathBuf::from("/"), item4.clone());
        assert_eq!(
            tree.get_closest(&PathBuf::from("/random/file1")).unwrap(),
            &item4
        );
    }

    #[test]
    fn iterator_works() {
        let mut tree: FsTree<Item> = FsTree::new();
        let item1 = Item {
            i: 1,
            s: String::from("s1"),
        };
        let item2 = Item {
            i: 2,
            s: String::from("s2"),
        };
        let item3 = Item {
            i: 3,
            s: String::from("s3"),
        };
        let item4 = Item {
            i: 4,
            s: String::from("s4"),
        };
        tree.insert(
            &PathBuf::from("/tmp/dir1/subdir1/subsubdir1"),
            item1.clone(),
        );
        tree.insert(
            &PathBuf::from("/tmp/dir2/subdir1/subsubdir2"),
            item2.clone(),
        );
        tree.insert(&PathBuf::from("/tmp/dir1"), item3.clone());
        tree.insert(&PathBuf::from("/"), item4.clone());

        let mut v: Vec<&Item> = Vec::new();
        for item in &tree {
            v.push(item);
        }
        assert_eq!(v.len(), 4);
        assert!(v.contains(&&item1));
        assert!(v.contains(&&item2));
        assert!(v.contains(&&item3));
        assert!(v.contains(&&item4));
    }
}
