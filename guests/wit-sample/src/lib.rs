wit_bindgen::generate!({
    world: "sample",
    path: "wit",
});

struct Component;

impl Guest for Component {
    fn add(a: i32, b: i32) -> i32 {
        a.wrapping_add(b)
    }
}

export!(Component);
