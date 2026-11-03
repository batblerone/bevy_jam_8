//! The shop module owns creation of the shop
//!

use bevy::prelude::*;

pub const TILE_SIZE: f32 = 32.0;
// put const LAYOUT_TEXT: str = [
//         ["####################"],
//         ["#BBBF..M..RRR..GGG.#"],
//         ["#..................#"],
//         ["#K..C....C....K.TZ.#"],
//         ["#........P.........#"],
//         ["#KKKKKKKSKKKKKKKKKK#"],
//         ["#.......Q..........#"],
//         ["#.......Q..........#"],
//         ["#.......Q..........#"],
//         ["########D###########"],
// ]

pub(super) fn plugin(app: &mut App) {
    todo!();
}


/*
Char	Meaning	
#	    wall
B	    ingredient crate	
M	    mortar	
C	    cauldron	
T	    trash	
G	    grow bed	
P	    witch spawn (floor)	
D	    customer door (floor)
.	    floor
F	    flask shelf
R	    drying rack
K	    counter
Z	    bed
S	    customer service counter
Q	    queue spot (floor)
*/