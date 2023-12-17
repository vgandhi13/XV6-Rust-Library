#[repr(C)]
enum ProcState {
    UNUSED,
    USED,
    SLEEPING,
    RUNNABLE,
    RUNNING,
    ZOMBIE,
}

#[repr(C)]
enum ProcColor {
    RED,
    BLACK,
}

#[repr(C)]
struct Spinlock {
    locked: bool,
    name: String,
    cpu: Option<Box<CPU>>,
}

#[repr(C)]
struct CPU {
    proc: Option<Box<Proc>>,
    context: Context,
    noff: i32,
    intena: i32,
}

#[repr(C)]
struct Proc {
    lock: Option<Box<Spinlock>>,
    state: ProcState,
    chan: *mut std::ffi::c_void,
    killed: i32,
    xstate: i32,
    pid: i32,
    parent: Option<Box<Proc>>,
    kstack: u64,
    sz: u64,
    pagetable: u64,
    trapframe: *mut Trapframe,
    context: Context,
    ofile: [Option<Box<File>>; NOFILE],
    cwd: Option<Box<Inode>>,
    name: [u8; 16],
    weight: i32,
    v_runtime: i32,
    niceness: i32,
    current_run: i32,
    max_exectime: i32,
    node_color: ProcColor,
    left_node: Option<Box<Proc>>,
    right_node: Option<Box<Proc>>,
    parent_node: Option<Box<Proc>>,
}

#[repr(C)]
struct RedBlackTree {
    num_of_nodes: i32,
    total_weight: i32,
    root: Option<Box<Proc>>,
    min_vruntime: Option<Box<Proc>>,
    lock: Spinlock,
    period: i32,
}




// attribute prevents the Rust compiler from applying name mangling to the function name, 
// ensuring that the function can be linked with C code using the specified name
#[no_mangle] 
pub extern "C" fn insert_process(tree: *mut RedBlackTree, p: *mut Proc) {

    // Convert raw pointers to mutable references
    // *tree dereferences the raw pointer, and then &mut takes a mutable reference to the value it points to. 
    // This is done inside an unsafe block because dereferencing raw pointers is an unsafe operation in Rust
    // mutable reference so upon return userinit still in c still has access to it, we are borrowingn ownership here
    let tree = unsafe { &mut *tree };
    let p = unsafe { &mut *p };

    if tree.num_of_nodes != 64 {
        // actually insert process into tree
        // I used take method here to move the value out of an Option while replacing it with Non
        tree.root = treenode_insertion(tree.root.take(), p); 
        //p passed above as a mutable reference
        //tree.root is  an Option<Box<Node>>
        if tree.num_of_nodes == 0 {
            if let Some(ref mut root) = tree.root {
                root.parentP = None;
            } //essentiaLLY Checks if tree.root is Some. If it is, it binds the value inside Some 
             // which is a mutable reference to the root node) to the variable root.
             // tree.root.unwrap() would panic if tree.root were None
        }

        updateInsertedProcessandTreeProperties(tree, p);

        // Now that the node has been added
        // Check for possible cases for Red-Black tree property violations
        // Recolor the tree starting from the position where the node was added
        recolorAndRotate(tree, p); 
    }
    // Return raw pointers to the modified tree and p
    (tree as *mut RedBlackTree, p as *mut Proc)
}




fn treenode_insertion(mut curProc: Option<Box<Proc>>, newProc: &mut Proc) -> Option<Box<Proc>> {
    newProc.node_color = ProcColor::RED;

    // If it is root or at leaf of tree
    if curProc.is_none() {
        return Some(Box::new(*newProc)); //Box essentially puts the result of treenode_insertion on heap
    }

    // Everything after root
    // Move process to the right of the current subtree
    // as_ref().unwrap().v_runtime allows us to obtain a reference to the content of the Option without consuming it, 
    // so we can still use curProc.as_mut().unwrap() later in the code without any issues. If not used then 
    // the memory will be owned not borrowed. as_ref() to obtain an immutable reference to the Proc inside the Option<Box<Proc>>. 
    // It allows us to access v_runtime but doesn't allow modifications. as_mut() to obtain a mutable reference to the Proc inside 
    // the Option<Box<Proc>>.  It allows you to both access and modify the v_runtime
    if curProc.as_ref().unwrap().v_runtime <= newProc.v_runtime {
        newProc.parent_node = Some(Box::new(curProc.unwrap()));
        curProc.as_mut().unwrap().right_node = treenode_insertion(curProc.unwrap().right_node, newProc);
    } else {
        newProc.parent_node = Some(Box::new(curProc.unwrap()));
        curProc.as_mut().unwrap().left_node = treenode_insertion(curProc.unwrap().left_node, newProc);
    }

    Some(Box::new(curProc.unwrap()))
}


fn get_minimum_vruntime_proc(traversing_process: Option<Box<Proc>>) -> Option<Box<Proc>> {
    // after this line traversing_process doesn't retain ownership, process is the owner
    // had it been this instead, if let Some(process) = traversing_process.as_mut()
    // traversin_process would've retainer ownership
    if let Some(mut process) = traversing_process {
        // remember we do not need to unwrap anything here
        // here ownership is transferred as well
        // this is because the function is not asking for a reference, but an owner
        // we don't do process.left_node.take() because that would make the left_node child None
        if let Some(left_node) = process.left_node {
            // If there is a left child, recursively call the function
            return get_minimum_vruntime_proc(Some(left_node));
        } else {
            // If there is no left child, return the current process
            return Some(process);
        }
    }
    // If traversing_process is None, return None
    None
}


fn recolor_and_rotate(tree: &mut RedBlackTree, rb_process: &mut Proc) {
    if let Some(parent) = rb_process.parent_node.as_mut() {
        if parent.node_color == ProcColor::RED {
            if let Some(grandparent) = rb_process.parent_node.parent_node.as_mut() {
                if let Some(uncle) = get_uncle(rb_process) {
                    if uncle.node_color == ProcColor::RED {
                        update_tree(tree, parent, uncle, grandparent);
                    } else if parent.left_node.is_some() {
                        l_and_lr_situations(tree, rb_process, parent, grandparent);
                    } else {
                        r_and_rl_situations(tree, rb_process, parent, grandparent);
                    }
                }
            }
        }
    }

    if let Some(root) = tree.root.as_mut() {
        root.node_color = ProcColor::BLACK;
    }
}
