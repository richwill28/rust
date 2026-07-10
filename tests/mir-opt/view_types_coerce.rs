// skip-filecheck
// Test that view type coercions, which is implemented via reborrowing,
// appear correctly in MIR.

struct MyStruct {
    field1: i32,
    field2: i32,
}

impl MyStruct {
    fn read_field1(&{field1} self) -> i32 {
        self.field1
    }

    fn read_field2(&{field2} self) -> i32 {
        self.field2
    }

    fn incr_field1(&mut {mut field1} self) -> i32 {
        self.field1 += 1;
        self.field1
    }

    fn incr_field2(&mut {mut field2} self) -> i32 {
        self.field2 += 1;
        self.field2
    }
}

fn take_view_ref(_x: &{field1} MyStruct) {}

// EMIT_MIR view_types_coerce.pass_normal_ref.built.after.mir
fn pass_normal_ref() {
    let x = MyStruct { field1: 10, field2: 20 };
    let r = &x;
    take_view_ref(r);
}

// EMIT_MIR view_types_coerce.pass_normal_mut_ref.built.after.mir
fn pass_normal_mut_ref() {
    let mut x = MyStruct { field1: 10, field2: 20 };
    let r = &mut x;
    take_view_ref(r);
}

// EMIT_MIR view_types_coerce.pass_direct_borrow.built.after.mir
fn pass_direct_borrow() {
    let x = MyStruct { field1: 10, field2: 20 };
    take_view_ref(&x);
}

// EMIT_MIR view_types_coerce.pass_direct_mut_borrow.built.after.mir
fn pass_direct_mut_borrow() {
    let mut x = MyStruct { field1: 10, field2: 20 };
    take_view_ref(&mut x);
}

// EMIT_MIR view_types_coerce.call_methods.built.after.mir
fn call_methods() {
    let x = MyStruct { field1: 10, field2: 20 };
    let _ = x.read_field1();
    let _ = x.read_field2();
}

// EMIT_MIR view_types_coerce.call_mut_methods.built.after.mir
fn call_mut_methods() {
    let mut x = MyStruct { field1: 10, field2: 20 };
    let _ = x.incr_field1();
    let _ = x.incr_field2();
}

fn main() {
    pass_normal_ref();
    pass_normal_mut_ref();
    pass_direct_borrow();
    pass_direct_mut_borrow();
    call_methods();
}
