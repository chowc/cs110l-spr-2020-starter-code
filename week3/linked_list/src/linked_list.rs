use std::borrow::BorrowMut;
use std::fmt;
use std::fmt::{Debug, Display};
use std::option::Option;

pub struct LinkedList<T> where T: Clone+PartialEq {
    head: Option<Box<Node<T>>>,
    size: usize,
}

#[derive(Debug)]
struct Node<T: Clone+PartialEq> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl <T: Clone+PartialEq> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {value, next }
    }
}

impl <T: Clone+PartialEq> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node::new(self.value.clone(), self.next.clone())
    }
}

impl <T: PartialEq+Clone> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl <T: Clone+PartialEq> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {head: None, size: 0}
    }
    
    pub fn get_size(&self) -> usize {
        self.size
    }
    
    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }
    
    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}


impl <T: Clone+PartialEq> fmt::Display for LinkedList<T> where T: Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                },
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl <T: Clone+PartialEq> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

impl <T: Clone+PartialEq> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        let mut list = LinkedList::<T>::new();
        list.head = self.head.clone();
        list.size = self.size;
        list
    }
}

impl <T: Clone+PartialEq+Debug> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }
        let mut head = &self.head;
        let mut other_head = &other.head;

        loop {

            match head {
                Some(node) => {
                    match other_head {
                        Some(other_node) => {
                            if node.ne(other_node) {
                                println!("compare node {:?}, other_node {:?}", node.value, other_node.value);
                                return false;
                            }
                            head = &node.next;
                            other_head = &other_node.next;
                        },
                        None => {
                            return false;
                        }
                    }
                },
                None => {
                    return other_head.is_none();
                },
            }
        }
    }
}

pub struct ListIterator<'a, T> where T: Clone+PartialEq {
    current: &'a Option<Box<Node<T>>>,
}

impl <'a, T: Clone+PartialEq> Iterator for ListIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let n = match self.current.as_ref() {
            Some(node) => {
                self.current = &node.next;
                Some(&node.value)
            },
            None => None,
        };
        n
    }
}

impl <'a, T: Clone+PartialEq> IntoIterator for &'a LinkedList<T> {
    type Item = &'a T;
    type IntoIter = ListIterator<'a, T>;

    fn into_iter(self) -> ListIterator<'a, T> {
        ListIterator{
            current: &self.head,
        }
    }
}