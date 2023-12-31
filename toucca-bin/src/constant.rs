pub const GET_SYNC_BOARD_VER: u8 = 0xA0;
pub const NEXT_READ: u8 = 0x72;
pub const GET_UNIT_BOARD_VER: u8 = 0xA8;
pub const MYSTERY_1: u8 = 0xA2;
pub const MYSTERY_2: u8 = 0x94;
pub const START_AUTO_SCAN: u8 = 0xC9;
pub const BEGIN_WRITE: u8 = 0x77;
pub const NEXT_WRITE: u8 = 0x20;
pub const BAD_IN_BYTE: u8 = 0x9A;
// pub const DATA_160: [u8; 8] = [160, 49, 57, 48, 53, 50, 51, 44];
pub const DATA_162: [u8; 3] = [162, 63, 29];
pub const DATA_148: [u8; 3] = [148, 0, 20];
pub const DATA_201: [u8; 3] = [201, 0, 73];
pub const SYNC_BOARD_VER: &str = "190523";
pub const UNIT_BOARD_VER: &str = "190514";
pub const READ_1: &str =
    "    0    0    1    2    3    4    5   15   15   15   15   15   15   11   11   11";
pub const READ_2: &str =
    "   11   11   11  128  103  103  115  138  127  103  105  111  126  113   95  100";
pub const READ_3: &str =
    "  101  115   98   86   76   67   68   48  117    0   82  154    0    6   35    4";
pub const UNIT_R: u8 = 118;
pub const UNIT_L: u8 = 104;
