use algae::glam::Vec2;
use algae::Operation;
use algae::operations::OrderedOperations;

fn main(){
    let op: Box<OrderedOperations<(), _>> = Box::new(algae_grammar::rexpr!{
        let d: Vec2 = Sub(Abs(Var(p, Vec2(0.0f32, 0.0f32))),  Var(ext, Vec2(1.0f32, 1.0f32)));
        let res: f32 = Add(Length(Max(d, Const(Vec2(0.0, 0.0)))),  Min(Max(VecSelectElement(d, 0), VecSelectElement(d, 1)), Const(0.0f32)));
	
        return res;
    });

    println!("Yay");
}
