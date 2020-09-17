use std::cmp;
use rand::Rng;

use tcod::colors::*;
use tcod::console::*;
use tcod::input::{self, Event, Key, Mouse};
use tcod::input::KeyCode::*;
use tcod::map::{FovAlgorithm, Map as FovMap};

mod object;
use object::*;

//mod map;
//:use map::*;

mod game;
use game::*;

//mod messages;
//use messages::*;

// 
// ===================== CONST
const DEBUG: bool = false;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const FPS_LIMIT: i32 = 20;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;

const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };

const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;

const PLAYER: usize = 0;
const INVENTORY_WIDTH: i32 = 50;

const HEAL_AMOUNT: i32 = 4;

struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    key: Key,
    mouse: Mouse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

type Map = Vec<Vec<Tile>>;

// ===================== GAME
//struct Game {
//    map: Map,
//    messages: Messages,
//}

// ===================== FUNCTIONS

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    let target_id = objects
        .iter()
        .position(|object| object.fighter.is_some() && object.pos() == (x, y));

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }

}

fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    if game.inventory.len() >= 26 {
        game.messages.add("Your inventory is full", RED);
    } else {
        let item = objects.swap_remove(object_id);
        game.messages.add(format!("You picked {}", item.name), GREEN);
        game.inventory.push(item);
    }
}

fn handle_keys(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) -> PlayerAction {
    use PlayerAction::*;
    let player_alive = objects[PLAYER].alive;
    //let key = tcod.root.wait_for_keypress(true);
    match (tcod.key, tcod.key.text(), player_alive) {
        (
            Key {
            code: Enter,
            alt: true,
            ..
            },
            _,
            _,
        ) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _, _) => Exit,
        (Key { code: Up, .. }, _, true)=> {
            player_move_or_attack(0,-1, game, objects);
            TookTurn
        }
        (Key { code: Down, .. }, _, true) => {
            player_move_or_attack(0,1, game, objects);
            TookTurn
        }
        (Key { code: Left, .. }, _, true) => {
            player_move_or_attack(-1,0, game, objects);
            TookTurn
        }
        (Key { code: Right, .. }, _, true) => {
            player_move_or_attack(1,0, game, objects);
            TookTurn
        }
        (Key { code: Text, ..}, "g", true) => {
            let item_id = objects
                .iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        (Key { code: Text, ..}, "i", true) => {
            let inventory_index = inventory_menu(
                &game.inventory, 
                "Press the key next to item to use it or any other th cancel\n",
                &mut tcod.root
                );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, tcod, game, objects);
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn 
    }
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    let names = objects
        .iter()
        .filter(|obj| obj.pos() == (x,y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();
    names.join(", ")
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) {
    let (monster_x, monster_y) = objects[monster_id].pos();
    // check if the monster can see us
    
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
}


fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    objects.iter().any(|object| object.blocks && object.pos() == (x,y))
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    //placing monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        if !is_blocked(x, y, map, objects) {

            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut orc = Object::new(x, y, 'o', DESATURATED_RED, "Orc", true);
                orc.fighter = Some(Fighter {
                    max_hp: 10,
                    hp: 10,
                    defense: 0,
                    power: 3,
                    on_death: DeathCallBack::Monster,
                });
                orc.ai = Some(Ai::Basic);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', DARKER_RED, "Troll", true);
                troll.fighter = Some(Fighter {
                    max_hp: 16,
                    hp: 16,
                    defense: 1,
                    power: 4,
                    on_death: DeathCallBack::Monster,
                });
                troll.ai = Some(Ai::Basic);
                troll
            };
            monster.alive = true;
            objects.push(monster);
        }

   }
   //placing objects
   let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);
   for _ in 0..num_items {
       let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
       let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
       if !is_blocked(x, y, map, objects) {
           let mut object = Object::new(x, y, 'b', VIOLET, "healing potion",  false);
           object.item = Some(Item::Heal);
           objects.push(object);
       }
   }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}


fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}


fn make_map(objects: &mut Vec<Object>) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    let mut rooms = vec![];
    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);
        let new_room = Rect::new(x, y, w, h);
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));
        if !failed {
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);
            let (new_x, new_y) = new_room.center();
            // check if vector is empty --> meaning this is the first room
            if rooms.is_empty() {
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut  map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut  map);
                }
            }
            rooms.push(new_room);
        }
    }

    map
}

fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color,
    ) {
    //bar width
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }
    panel.set_default_foreground(WHITE);
    panel.print_ex(
        x + total_width / 2,
        y, 
        BackgroundFlag::None,
        TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
        );
}


fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    // Draw all objects from the list
    let mut to_draw: Vec<_> = objects
        .iter().filter(|o| tcod.fov.is_in_fov(o.x, o.y))
        .collect();

    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

        // Set all tiles and set bckg color
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let color = match(visible, wall) {
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            let explored = &mut game.map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }
                if *explored || DEBUG {
                    tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
        }
    }
    
    blit(&tcod.con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), &mut tcod.root, (0,0), 1.0, 1.0,);

    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    //player stats
   tcod.panel.set_default_background(BLACK);
   tcod.panel.clear();
   let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
   let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
   render_bar(
       &mut tcod.panel,
       1,
       1,
       BAR_WIDTH,
       "HP",
       hp,
       max_hp,
       LIGHT_RED,
       DARKER_RED,
       );

   tcod.panel.set_default_foreground(LIGHT_GREY);
   tcod.panel.print_ex(
       1,
       0,
       BackgroundFlag::None,
       TextAlignment::Left,
       get_names_under_mouse(tcod.mouse, objects, &tcod.fov),
   );

   //message logs
   let mut y = MSG_HEIGHT as i32;
   for &(ref msg, color) in game.messages.iter().rev() {
       let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
       y -= msg_height;
       if y < 0 {
           break;
       }
       tcod.panel.set_default_foreground(color);
       tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
   }
   blit(&tcod.panel, (0,0), (SCREEN_WIDTH, PANEL_HEIGHT), &mut tcod.root, (0,PANEL_Y), 1.0, 1.0);
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, seconde_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut seconde_slice[0])
    } else {
        (&mut seconde_slice[0], &mut first_slice[second_index])
    }
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(options.len() <= 26, "Max 26 options");
    //total height 
    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(WHITE);
    window.print_rect_ex(
        0,
        0,
        width,
        height,
        BackgroundFlag::None,
        TextAlignment::Left,
        header,
        );

    // print all options
    for(index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text
            );
    }
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(&window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);
    root.flush();
    let key = root.wait_for_keypress(true);
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };
    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);
    if inventory.len() > 0 {
        inventory_index 
    } else {
        None
    }
}

fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            object::Item::Heal => cast_heal,
        };
        match on_use(inventory_id, tcod, game, objects) {
            UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(format!("The {} cannot be used", game.inventory[inventory_id].name),
        WHITE,
        );
    }
}

fn cast_heal(
    _inventory_id: usize, 
    _tcod: &mut Tcod, 
    game: &mut Game, 
    objects: &mut [Object],
    ) -> UseResult {
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health", ORANGE);
            return UseResult::Cancelled;
        }
        game.messages.add("You wounds starts to feel better!", LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

// ===================== MAIN
fn main() {
    tcod::system::set_fps(FPS_LIMIT);

    // General Window Setup
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("LIBTCOD Tutorial")
        .init();

    let mut tcod = Tcod {
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        key: Default::default(),
        mouse: Default::default(),
    };

    // player_position 
   
    let mut player = Object::new(0, 0, '@', WHITE, "player", true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: DeathCallBack::Player,
    });

    let mut objects = vec![player];
    let mut game = Game {
        map: make_map(&mut objects),
        messages: Messages::new(),
        inventory: vec![],
    };
    
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x, 
                y, 
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
                );
        }
    }
    // force the FOV to be recompute
    let mut previous_player_position = (-1, -1);

    game.messages.add(
        "Welcome stranger! prepare to perish in the tombs of the acient kings",
        RED
        );
    
    // game_loop 
    while !tcod.root.window_closed() {
        tcod.con.clear();
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }
        render_all(&mut tcod, &mut game, &objects, fov_recompute);
        tcod.root.flush();
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(&mut tcod, &mut game, &mut objects);
        if player_action == PlayerAction::Exit {
            break;
        }
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
           for id in 0..objects.len() {
               if objects[id].ai.is_some() {
                   ai_take_turn(id, &tcod, &mut game, &mut objects);
               }
           }
        }
    }

}