use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<u32> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i);
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string()); // ToString impl for anything impl Display

    let clone_list = list.clone();
    println!("clone {}", clone_list);
    println!("two list equal: {}", list.eq(&clone_list));
    list.push_front(100);
    println!("origin {}, clone {}", list, clone_list);
    assert!(true, list.ne(&clone_list));
    let _ = list.pop_front();
    assert!(true, list.eq(&clone_list));

    // If you implement iterator trait:
    for val in &list {
       println!("{}", val);
    }
}
