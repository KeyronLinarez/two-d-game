
pub struct GameState{
    // is the game in progress? no -> title screen
    pub running: bool,
    // is there a block bouncing side to side at top
    pub waiting: bool,
    // false if right, true if left moving animation
    pub direction: bool,
    // is there a block falling to the tower stack
    pub falling: bool,
    // how many blocks have fallen
    pub num_stacked: u32,
    // where is the left border for where blocks can stack
    pub left_border: f32,
    // where is the right border for where blocks can stack
    pub right_border: f32,
    // how many sprite blocks wide do we drop next
    pub drop_sprite_blocks: u16
}
impl GameState {
// add all necessary functions

}




pub fn init_game_state() -> GameState {
    // any necessary functions

    GameState {
        //sprite vector
        
        // is the game in progress? no -> title screen
        running : true,
        // is there a block bouncing side to side at top
        waiting : true,
        // false if right, true if left moving animation
        direction : false,
        // is there a block falling to the tower stack
        falling : false,
        // how many blocks have fallen
        num_stacked : 0,
        // where is the left border for where blocks can stack
        left_border : 0.0,
        // where is the right border for where blocks can stack
        right_border : 1080.0,
        // how many sprite blocks wide do we drop next
        drop_sprite_blocks : 10
    }
}
