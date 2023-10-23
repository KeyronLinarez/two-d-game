pub struct GameState{
    // Screen number: 0 = Title, 1 = Block Game, 2 = Black GO, 3 = Space Game, 4 = Space GO
    pub screen: usize,
    // level
    pub level: usize,
    // is the game in progress? no -> title screen
    pub running: bool,
    // is there a block bouncing side to side at top
    pub waiting: bool,
    // false if right, true if left moving animation
    pub direction: bool,
    // is there a block falling to the tower stack
    pub falling: bool,
    // how many blocks have fallen
    pub num_stacked: usize,
    // how many sprites have been used in the vec
    pub sprites_used: usize,
    // where is the left border for where blocks can stack
    pub left_border: f32,
    // where is the right border for where blocks can stack
    pub right_border: f32,
    // how many sprite blocks wide do we drop next
    pub drop_sprite_blocks: usize,
    // speed of blocks moving 
    pub speed:usize
}
impl GameState {
// add all necessary functions
    // pub fn reset(game_mode: u8){
        
    // }
}

pub fn init_game_state() -> GameState {
    // any necessary functions

    GameState {
        // Screen number: 0 = Title, 1 = Block Game, 2 = Block Setup, 3 = Black GO, 4 = Space Game, 5 = Space Setup, 6 = Space GO
        screen : 0,
        //level
        level : 1,
        // is the game in progress? no -> title screen
        running : true,
        // is there a block bouncing side to side at top
        waiting : false,
        // false if right, true if left moving animation
        direction : false,
        // is there a block falling to the tower stack
        falling : false,
        // how many blocks have fallen
        num_stacked : 0,
        // how many sprites have been used in the vec
        sprites_used: 0,
        // where is the left border for where blocks can stack
        left_border : 0.0,
        // where is the right border for where blocks can stack
        right_border : 1080.0,
        // how many sprite blocks wide do we drop next
        drop_sprite_blocks : 5,
        // speed of blocks moving
        speed: 4,
    }
}
