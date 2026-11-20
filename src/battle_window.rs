use crate::collection_window;
use crate::windows::{game_icon, wide};
use hundred_math::battle::{Battle, Outcome, OPPONENTS};
use hundred_math::collection::{unlocked_coins, COIN_COUNT};
use hundred_math::{Operation, SIDE};
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{EM_SETLIMITTEXT, EM_SETSEL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, SetActiveWindow, SetFocus, VK_CONTROL, VK_DOWN, VK_LEFT, VK_RETURN,
    VK_RIGHT, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ATTACK: usize = 1;
const PRACTICE: usize = 2;
const START: usize = 3;
const OPPONENT: usize = 4;
const OPERATION: usize = 5;
const ANSWER: usize = 6;
pub const COLLECTION: usize = 7;
const HOME: usize = 8;
const ENTER_BATTLE: usize = 9;
const HOME_PRACTICE: usize = 10;
const CUTIN_CONTINUE: usize = 11;
const START_ART: &[u8; 564 * 376 * 3] = include_bytes!("../assets/start.bgr");
const TIMER: usize = 1;
const MOVE_CURSOR: u32 = WM_APP + 1;
const GRID_X: i32 = 19;
const GRID_Y: i32 = 198;
const CELL_X: i32 = 51;
const CELL_Y: i32 = 30;
const PORTRAITS: &[u8; 12 * 72 * 96 * 4] = include_bytes!("../assets/opponents.bgra");
const WORRIED: &[u8; 12 * 72 * 96 * 4] = include_bytes!("../assets/opponents-worried.bgra");
const LAUGH: &[u8; 12 * 72 * 96 * 4] = include_bytes!("../assets/opponents-laugh.bgra");

#[derive(Clone, Copy)]
enum Mood {
    Neutral,
    Worried,
    Laugh,
}

#[derive(Clone, Copy)]
enum Cutin {
    Start,
    Win,
    Loss,
}

struct Round {
    game: Battle,
    started: Instant,
}

impl Round {
    fn elapsed(&self) -> u32 {
        self.started.elapsed().as_millis().min(u32::MAX as u128) as u32
    }
}

struct BattleWindow {
    opponent_picker: HWND,
    operation_picker: HWND,
    answer: HWND,
    attack: HWND,
    progress: HWND,
    stats: HWND,
    question: HWND,
    note: HWND,
    start: HWND,
    font: HFONT,
    points_label: HWND,
    points: u32,
    opponent: usize,
    round: Option<Round>,
    home: bool,
    home_buttons: [HWND; 2],
    battle_controls: Vec<HWND>,
    title_font: HFONT,
    collection: HWND,
    cutin_button: HWND,
    cutin: Option<(Cutin, Instant)>,
    mood: Mood,
    reaction_until: Option<Instant>,
    opponent_lines: u32,
    finish_time: Option<u32>,
}

fn box_rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

unsafe fn set_text(hwnd: HWND, text: &str) {
    SetWindowTextW(hwnd, wide(text).as_ptr());
}

impl BattleWindow {
    unsafe fn new(hwnd: HWND) -> Box<Self> {
        let font = CreateFontW(
            -17,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            0,
            0,
            wide("Yu Gothic UI").as_ptr(),
        );
        let mut battle_controls = Vec::new();
        let mut control = |class: &str, label: &str, style, id, (x, y, w, h)| {
            let handle = CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(label).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                x,
                y,
                w,
                h,
                hwnd,
                id as HMENU,
                null_mut(),
                null(),
            );
            SendMessageW(handle, WM_SETFONT, font as WPARAM, 1);
            battle_controls.push(handle);
            handle
        };
        control("STATIC", "対戦相手", 0, 0, (18, 10, 160, 24));
        let opponent_picker = control(
            "COMBOBOX",
            "",
            WS_TABSTOP | CBS_DROPDOWNLIST as u32,
            OPPONENT,
            (18, 34, 161, 310),
        );
        for name in OPPONENTS {
            SendMessageW(
                opponent_picker,
                CB_ADDSTRING,
                0,
                wide(name).as_ptr() as LPARAM,
            );
        }
        SendMessageW(opponent_picker, CB_SETCURSEL, 0, 0);
        control("STATIC", "計算の種類", 0, 0, (190, 10, 130, 24));
        let operation_picker = control(
            "COMBOBOX",
            "",
            WS_TABSTOP | CBS_DROPDOWNLIST as u32,
            OPERATION,
            (190, 34, 130, 125),
        );
        for name in ["＋ たし算", "− ひき算", "× かけ算"] {
            SendMessageW(
                operation_picker,
                CB_ADDSTRING,
                0,
                wide(name).as_ptr() as LPARAM,
            );
        }
        SendMessageW(operation_picker, CB_SETCURSEL, 0, 0);
        let start = control("BUTTON", "対戦開始", WS_TABSTOP, START, (329, 33, 132, 28));
        control(
            "BUTTON",
            "れんしゅう",
            WS_TABSTOP,
            PRACTICE,
            (470, 33, 112, 28),
        );
        let progress = control("STATIC", "", 0, 0, (121, 77, 461, 24));
        let stats = control("STATIC", "", 0, 0, (121, 106, 461, 24));
        let question = control("STATIC", "", 0, 0, (121, 134, 202, 26));
        let answer = control(
            "EDIT",
            "",
            WS_TABSTOP | WS_BORDER | ES_CENTER as u32,
            ANSWER,
            (330, 132, 70, 28),
        );
        SendMessageW(answer, EM_SETLIMITTEXT, 5, 0);
        let attack = control("BUTTON", "答える", WS_TABSTOP, ATTACK, (409, 131, 79, 30));
        let note = control("STATIC", "", 0, 0, (121, 164, 461, 25));
        let points_label = control("STATIC", "", 0, 0, (19, 538, 392, 25));
        let collection = control(
            "BUTTON",
            "コイン図鑑",
            WS_TABSTOP,
            COLLECTION,
            (450, 532, 130, 29),
        );
        control("BUTTON", "ホーム", WS_TABSTOP, HOME, (340, 532, 100, 29));
        let home_buttons = [
            control(
                "BUTTON",
                "対戦をはじめる",
                WS_TABSTOP,
                ENTER_BATTLE,
                (19, 519, 224, 40),
            ),
            control(
                "BUTTON",
                "れんしゅう",
                WS_TABSTOP,
                HOME_PRACTICE,
                (254, 519, 178, 40),
            ),
        ];
        let cutin_button = control(
            "BUTTON",
            "つづける",
            WS_TABSTOP,
            CUTIN_CONTINUE,
            (286, 367, 140, 40),
        );
        ShowWindow(cutin_button, SW_HIDE);
        battle_controls.retain(|handle| {
            !home_buttons.contains(handle) && *handle != collection && *handle != cutin_button
        });
        let title_font = CreateFontW(
            -32,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            0,
            0,
            wide("Yu Gothic UI").as_ptr(),
        );
        let mut state = Box::new(Self {
            opponent_picker,
            operation_picker,
            answer,
            attack,
            progress,
            stats,
            question,
            note,
            start,
            font,
            points_label,
            points: collection_window::load_points(),
            opponent: 0,
            round: None,
            home: true,
            home_buttons,
            battle_controls,
            title_font,
            collection,
            cutin_button,
            cutin: None,
            mood: Mood::Neutral,
            reaction_until: None,
            opponent_lines: 0,
            finish_time: None,
        });
        state.reset(hwnd);
        state.show_points();
        state.show_home(hwnd, true);
        state
    }

    unsafe fn show_home(&mut self, hwnd: HWND, home: bool) {
        self.home = home;
        if home {
            self.reset(hwnd);
        }
        for handle in &self.battle_controls {
            ShowWindow(*handle, if home { SW_HIDE } else { SW_SHOWNA });
        }
        for handle in self.home_buttons {
            ShowWindow(handle, if home { SW_SHOWNA } else { SW_HIDE });
        }
        if IsWindowVisible(hwnd) != 0 {
            SetFocus(if home {
                self.home_buttons[0]
            } else {
                self.opponent_picker
            });
        }
        InvalidateRect(hwnd, null(), 1);
    }

    unsafe fn paint_home(&self, dc: HDC) {
        let mut cover = box_rect(0, 0, 600, 566);
        let paper = CreateSolidBrush(0x00dcf3fc);
        FillRect(dc, &cover, paper);
        DeleteObject(paper);

        let previous_font = SelectObject(dc, self.title_font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0x00465415);
        cover = box_rect(19, 12, 581, 69);
        DrawTextW(
            dc,
            wide("計算バトルへようこそ！").as_ptr(),
            -1,
            &mut cover,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        SelectObject(dc, self.font);
        cover = box_rect(19, 461, 581, 507);
        DrawTextW(
            dc,
            wide("相手より先に100マスを埋めよう。勝つたびにコインが集まるよ！").as_ptr(),
            -1,
            &mut cover,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        SelectObject(dc, previous_font);

        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 564;
        info.bmiHeader.biHeight = -376;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 24;
        info.bmiHeader.biCompression = BI_RGB;
        StretchDIBits(
            dc,
            18,
            76,
            564,
            376,
            0,
            0,
            564,
            376,
            START_ART.as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }

    unsafe fn show_points(&self) {
        let coins = unlocked_coins(self.points);
        set_text(
            self.points_label,
            &format!("{} pt    {coins} / {COIN_COUNT} 枚", self.points),
        );
    }

    unsafe fn win_message(&mut self) -> String {
        let award = self
            .round
            .as_mut()
            .map_or(0, |round| round.game.take_reward());
        if award == 0 {
            return "勝ち！ よくできました。".to_owned();
        }
        let previous = self.points.max(collection_window::load_points());
        self.points = previous.saturating_add(award);
        let saved = collection_window::save_points(self.points);
        self.show_points();
        let before = unlocked_coins(previous);
        let mut message = format!("勝ち！ +{award} pt");
        if unlocked_coins(self.points) > before {
            if let Some(name) = collection_window::coin_name(before) {
                message.push_str(&format!("・{name}のコインを獲得"));
            }
        }
        if !saved {
            message.push_str("（保存できませんでした）");
        }
        message
    }

    unsafe fn enable_answer(&self, enabled: bool) {
        EnableWindow(self.answer, enabled.into());
        EnableWindow(self.attack, enabled.into());
    }

    unsafe fn reset(&mut self, hwnd: HWND) {
        KillTimer(hwnd, TIMER);
        self.round = None;
        self.mood = Mood::Neutral;
        self.reaction_until = None;
        self.opponent_lines = 0;
        self.finish_time = None;
        self.opponent = SendMessageW(self.opponent_picker, CB_GETCURSEL, 0, 0)
            .clamp(0, (OPPONENTS.len() - 1) as isize) as usize;
        self.enable_answer(false);
        set_text(self.answer, "");
        set_text(self.question, "対戦開始を押してね");
        set_text(self.start, "対戦開始");
        set_text(self.note, "");
        self.refresh();
        InvalidateRect(hwnd, null(), 1);
    }

    unsafe fn start(&mut self, hwnd: HWND) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let seed = stamp as u32 ^ (stamp >> 32) as u32;
        let operation = SendMessageW(self.operation_picker, CB_GETCURSEL, 0, 0) as u32;
        self.round = Some(Round {
            game: Battle::new(seed, Operation::from_index(operation), self.opponent),
            started: Instant::now(),
        });
        self.mood = Mood::Neutral;
        self.reaction_until = None;
        self.opponent_lines = 0;
        self.finish_time = None;
        self.enable_answer(false);
        self.show_cutin(hwnd, Cutin::Start, 1500);
        SetTimer(hwnd, TIMER, 100, None);
    }

    unsafe fn show_cutin(&mut self, hwnd: HWND, frame: Cutin, duration: u64) {
        self.cutin = Some((frame, Instant::now() + Duration::from_millis(duration)));
        for &handle in self.battle_controls.iter().chain([&self.collection]) {
            ShowWindow(handle, SW_HIDE);
        }
        ShowWindow(self.cutin_button, SW_SHOWNA);
        SetFocus(self.cutin_button);
        InvalidateRect(hwnd, null(), 1);
    }

    unsafe fn finish_cutin(&mut self, hwnd: HWND) {
        let Some((frame, _)) = self.cutin.take() else {
            return;
        };
        ShowWindow(self.cutin_button, SW_HIDE);
        for &handle in self.battle_controls.iter().chain([&self.collection]) {
            ShowWindow(handle, SW_SHOWNA);
        }
        if matches!(frame, Cutin::Start) {
            if let Some(round) = &mut self.round {
                round.started = Instant::now();
            }
            self.enable_answer(true);
            set_text(self.start, "対戦をやり直す");
            set_text(self.answer, "");
            set_text(self.note, "クリック・矢印キーでマスを選べます。");
            SetFocus(self.answer);
        } else {
            KillTimer(hwnd, TIMER);
            SetFocus(self.start);
        }
        self.refresh();
        InvalidateRect(hwnd, null(), 1);
    }

    unsafe fn refresh(&self) {
        let (player, cpu, seconds) = self.round.as_ref().map_or((0, 0, 0), |round| {
            (
                round.game.solved_count,
                round.game.opponent_solved,
                self.finish_time.unwrap_or_else(|| round.elapsed()) / 1_000,
            )
        });
        set_text(
            self.progress,
            &format!(
                "あなた {player} / 100     {} {cpu} / 100",
                OPPONENTS[self.opponent]
            ),
        );
        let race_status = if self.finish_time.is_some() {
            "対戦終了"
        } else {
            "タイム"
        };
        set_text(
            self.stats,
            &format!("{race_status}   {:02}:{:02}", seconds / 60, seconds % 60),
        );
        if let Some(round) = &self.round {
            let (row, col) = round.game.question();
            set_text(
                self.question,
                &format!("{row} {} {col} = ?", round.game.operation.symbol()),
            );
        }
    }

    unsafe fn react(&mut self, hwnd: HWND, mood: Mood) {
        self.mood = mood;
        self.reaction_until = Some(Instant::now() + Duration::from_millis(2200));
        InvalidateRect(hwnd, &box_rect(23, 80, 104, 188), 0);
    }

    unsafe fn finish_or_refresh(&mut self, hwnd: HWND) {
        let Some(round) = &self.round else {
            return;
        };
        let outcome = round.game.outcome;
        let lines = round.game.opponent_solved / 10;
        if outcome == Outcome::Playing {
            if lines > self.opponent_lines {
                self.opponent_lines = lines;
                set_text(
                    self.note,
                    &format!("{}のラインアタック！", OPPONENTS[self.opponent]),
                );
                self.react(hwnd, Mood::Laugh);
            }
            self.refresh();
            return;
        }

        self.finish_time = Some(round.elapsed());
        self.enable_answer(false);
        set_text(self.start, "もう一度");
        let (message, frame, mood) = if outcome == Outcome::Win {
            (self.win_message(), Cutin::Win, Mood::Worried)
        } else {
            (
                "相手が先に100マスを埋めました。".to_owned(),
                Cutin::Loss,
                Mood::Laugh,
            )
        };
        self.mood = mood;
        set_text(self.note, &message);
        self.refresh();
        self.show_cutin(hwnd, frame, 2600);
    }

    unsafe fn tick(&mut self, hwnd: HWND) {
        if let Some((_, until)) = self.cutin {
            if Instant::now() >= until {
                self.finish_cutin(hwnd);
            }
            return;
        }
        let Some(round) = &mut self.round else {
            return;
        };
        if round.game.outcome != Outcome::Playing {
            return;
        }
        round.game.tick(round.elapsed());
        if self
            .reaction_until
            .is_some_and(|until| Instant::now() >= until)
        {
            self.mood = Mood::Neutral;
            self.reaction_until = None;
            InvalidateRect(hwnd, &box_rect(23, 80, 104, 188), 0);
        }
        self.finish_or_refresh(hwnd);
    }

    unsafe fn submit(&mut self, hwnd: HWND) {
        let Some(round) = &mut self.round else {
            return;
        };
        if round.game.outcome != Outcome::Playing {
            return;
        }
        let mut utf16 = [0; 6];
        let size = GetWindowTextW(self.answer, utf16.as_mut_ptr(), utf16.len() as i32) as usize;
        let input: String = String::from_utf16_lossy(&utf16[..size])
            .trim()
            .chars()
            .map(|ch| match ch {
                '０'..='９' => char::from(b'0' + (ch as u32 - '０' as u32) as u8),
                '−' | '－' => '-',
                _ => ch,
            })
            .collect();
        let Ok(value) = input.parse::<i32>() else {
            set_text(self.note, "整数を入力してね。");
            SetFocus(self.answer);
            return;
        };

        match round.game.submit(value, round.elapsed()) {
            Some(true) => {
                let lines = round.game.last_lines;
                if lines > 0 {
                    set_text(self.note, &format!("{lines}ライン完成！ ラインアタック！"));
                    self.react(hwnd, Mood::Worried);
                } else {
                    set_text(self.note, "正解！");
                }
                set_text(self.answer, "");
                SetFocus(self.answer);
                InvalidateRect(hwnd, null(), 1);
            }
            Some(false) => {
                set_text(self.note, "おしい！ 同じマスにもう一度答えよう。");
                SetFocus(self.answer);
                SendMessageW(self.answer, EM_SETSEL, 0, -1);
            }
            None => {}
        }
        self.finish_or_refresh(hwnd);
    }

    unsafe fn select_square(&mut self, hwnd: HWND, index: Option<usize>, direction: (i32, i32)) {
        let Some(round) = &mut self.round else {
            return;
        };
        let selected = match index {
            Some(cell) => round.game.select(cell),
            None => round.game.move_selection(direction.0, direction.1),
        };
        if selected {
            set_text(self.answer, "");
            SetFocus(self.answer);
            self.refresh();
            InvalidateRect(hwnd, null(), 0);
        }
    }

    unsafe fn paint_portrait(&self, dc: HDC, rect: RECT) {
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 72;
        info.bmiHeader.biHeight = -96;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let portraits = match self.mood {
            Mood::Neutral => PORTRAITS,
            Mood::Worried => WORRIED,
            Mood::Laugh => LAUGH,
        };
        let at = self.opponent * 72 * 96 * 4;
        StretchDIBits(
            dc,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            0,
            0,
            72,
            96,
            portraits[at..].as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }

    unsafe fn paint_cutin(&self, dc: HDC, frame: Cutin) {
        let backdrop = CreateSolidBrush(0x002b3815);
        let paper = CreateSolidBrush(0x00d6f3ff);
        FillRect(dc, &box_rect(0, 0, 600, 566), backdrop);
        FillRect(dc, &box_rect(24, 146, 576, 426), paper);
        DeleteObject(backdrop);
        DeleteObject(paper);
        self.paint_portrait(dc, box_rect(45, 172, 207, 388));
        let title = match frame {
            Cutin::Start => "百マスレース！",
            Cutin::Win => "100マス達成！",
            Cutin::Loss => "相手の勝ち！",
        };
        let old = SelectObject(dc, self.title_font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0x00333e13);
        let mut area = box_rect(223, 184, 559, 270);
        DrawTextW(
            dc,
            wide(title).as_ptr(),
            -1,
            &mut area,
            DT_VCENTER | DT_SINGLELINE,
        );
        SelectObject(dc, self.font);
        let detail = match (frame, &self.round) {
            (Cutin::Start, _) => format!("相手は {}。よーい、スタート！", OPPONENTS[self.opponent]),
            (_, Some(round)) => format!(
                "あなた {} マス ／ {} {} マス",
                round.game.solved_count, OPPONENTS[self.opponent], round.game.opponent_solved
            ),
            _ => String::new(),
        };
        let mut area = box_rect(223, 270, 559, 356);
        DrawTextW(dc, wide(&detail).as_ptr(), -1, &mut area, DT_WORDBREAK);
        SelectObject(dc, old);
    }

    unsafe fn paint(&self, hwnd: HWND) {
        let mut paint: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        if self.home {
            self.paint_home(dc);
            EndPaint(hwnd, &paint);
            return;
        }
        if let Some((frame, _)) = self.cutin {
            self.paint_cutin(dc, frame);
            EndPaint(hwnd, &paint);
            return;
        }
        self.paint_portrait(dc, box_rect(23, 80, 104, 188));

        if let Some(round) = &self.round {
            let previous_font = SelectObject(dc, self.font);
            SetBkMode(dc, TRANSPARENT as i32);
            let game = &round.game;
            let selected = (game.outcome == Outcome::Playing)
                .then_some((game.cell / SIDE + 1, game.cell % SIDE + 1));
            let header = CreateSolidBrush(0x00e7efe4);
            let current = CreateSolidBrush(0x00bff0ff);
            let crosshair = CreateSolidBrush(0x00ebf9f5);
            let active_axis = CreateSolidBrush(0x00a4dff6);
            let completed = CreateSolidBrush(0x00526417);
            let solved = CreateSolidBrush(0x00e0ebd7);
            for row in 0..=SIDE {
                for col in 0..=SIDE {
                    let x = GRID_X + CELL_X * col as i32;
                    let y = GRID_Y + CELL_Y * row as i32;
                    let mut rect = box_rect(x, y, x + CELL_X - 1, y + CELL_Y - 1);
                    let (in_row, in_col) =
                        selected.map_or((false, false), |(r, c)| (r == row, c == col));
                    let (value, brush) = match (row, col) {
                        (0, 0) => (game.operation.symbol().to_owned(), header),
                        (0, col) => (
                            game.numbers[SIDE + col - 1].to_string(),
                            if game.line_complete(col - 1, true) {
                                completed
                            } else if in_col {
                                active_axis
                            } else {
                                header
                            },
                        ),
                        (row, 0) => (
                            game.numbers[row - 1].to_string(),
                            if game.line_complete(row - 1, false) {
                                completed
                            } else if in_row {
                                active_axis
                            } else {
                                header
                            },
                        ),
                        (row, col) => {
                            let index = (row - 1) * SIDE + col - 1;
                            let answered = game.is_solved(index);
                            let brush = if in_row && in_col {
                                current
                            } else if answered {
                                solved
                            } else if in_row || in_col {
                                crosshair
                            } else {
                                GetSysColorBrush(COLOR_WINDOW)
                            };
                            let value = if answered {
                                game.operation
                                    .answer(game.numbers[row - 1], game.numbers[SIDE + col - 1])
                                    .to_string()
                            } else {
                                String::new()
                            };
                            (value, brush)
                        }
                    };
                    SetTextColor(
                        dc,
                        GetSysColor(if brush == completed {
                            COLOR_WINDOW
                        } else {
                            COLOR_WINDOWTEXT
                        }),
                    );
                    FillRect(dc, &rect, brush);
                    FrameRect(dc, &rect, GetSysColorBrush(COLOR_3DLIGHT));
                    if !value.is_empty() {
                        let utf16 = wide(&value);
                        DrawTextW(
                            dc,
                            utf16.as_ptr(),
                            -1,
                            &mut rect,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                        );
                    }
                }
            }
            SelectObject(dc, previous_font);
            for brush in [header, current, crosshair, active_axis, completed, solved] {
                DeleteObject(brush);
            }
        }
        EndPaint(hwnd, &paint);
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut BattleWindow;
    match message {
        WM_CREATE => {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                Box::into_raw(BattleWindow::new(hwnd)) as isize,
            );
            return 0;
        }
        WM_CLOSE => {
            let owner = GetWindow(hwnd, GW_OWNER);
            EnableWindow(owner, 1);
            DestroyWindow(hwnd);
            SetActiveWindow(owner);
            SetFocus(GetDlgItem(owner, super::windows::OPEN_BATTLE as i32));
            return 0;
        }
        WM_NCDESTROY if !state.is_null() => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let state = Box::from_raw(state);
            DeleteObject(state.font);
            DeleteObject(state.title_font);
        }
        WM_PAINT if !state.is_null() => {
            (*state).paint(hwnd);
            return 0;
        }
        WM_COMMAND if !state.is_null() => {
            let id = w & 0xffff;
            let event = (w >> 16) as u32;
            if matches!(id, PRACTICE | HOME_PRACTICE) && event == BN_CLICKED {
                SendMessageW(hwnd, WM_CLOSE, 0, 0);
                return 0;
            }
            // Control updates can synchronously send another window message.
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            match (id, event) {
                (OPPONENT | OPERATION, CBN_SELCHANGE) => (*state).reset(hwnd),
                (HOME, BN_CLICKED) => (*state).show_home(hwnd, true),
                (ENTER_BATTLE, BN_CLICKED) => (*state).show_home(hwnd, false),
                (CUTIN_CONTINUE, BN_CLICKED) => (*state).finish_cutin(hwnd),
                (START, BN_CLICKED) => (*state).start(hwnd),
                (ATTACK, BN_CLICKED) => (*state).submit(hwnd),
                (COLLECTION, BN_CLICKED) => {
                    if (*state)
                        .round
                        .as_ref()
                        .is_some_and(|round| round.game.outcome == Outcome::Playing)
                    {
                        (*state).reset(hwnd);
                    }
                    (*state).points = (*state).points.max(collection_window::load_points());
                    (*state).show_points();
                    collection_window::open(hwnd, (*state).points);
                }
                _ => {}
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            return 0;
        }
        WM_LBUTTONDOWN | MOVE_CURSOR if !state.is_null() && (*state).cutin.is_none() => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if message == MOVE_CURSOR {
                let direction = match w as u16 {
                    VK_LEFT => (-1, 0),
                    VK_RIGHT => (1, 0),
                    VK_UP => (0, -1),
                    VK_DOWN => (0, 1),
                    _ => (0, 0),
                };
                (*state).select_square(hwnd, None, direction);
            } else {
                let x = (l as i16 as i32) - GRID_X;
                let y = ((l >> 16) as i16 as i32) - GRID_Y;
                let col = x / CELL_X;
                let row = y / CELL_Y;
                if (1..=SIDE as i32).contains(&row) && (1..=SIDE as i32).contains(&col) {
                    (*state).select_square(
                        hwnd,
                        Some((row as usize - 1) * SIDE + col as usize - 1),
                        (0, 0),
                    );
                }
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            return 0;
        }
        WM_TIMER if w == TIMER && !state.is_null() => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            (*state).tick(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, w, l)
}

pub unsafe fn open(owner: HWND) {
    let class = wide("HundredMathBattle");
    let instance = GetModuleHandleW(null());
    RegisterClassW(&WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: instance,
        lpszClassName: class.as_ptr(),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hIcon: game_icon(instance),
        hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
        ..std::mem::zeroed()
    });
    let style = WS_CAPTION | WS_SYSMENU;
    let mut rect = box_rect(0, 0, 600, 566);
    AdjustWindowRectEx(&mut rect, style, 0, WS_EX_CONTROLPARENT);
    let mut owner_rect = std::mem::zeroed();
    GetWindowRect(owner, &mut owner_rect);
    let window = CreateWindowExW(
        WS_EX_CONTROLPARENT,
        class.as_ptr(),
        wide("100マス計算").as_ptr(),
        style,
        owner_rect.left,
        owner_rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        owner,
        null_mut(),
        instance,
        null(),
    );
    if !window.is_null() {
        EnableWindow(owner, 0);
        ShowWindow(window, SW_SHOW);
        SetFocus(GetDlgItem(window, ENTER_BATTLE as i32));
    }
}

pub unsafe fn translate(window: HWND, message: &MSG) -> bool {
    if message.message == WM_KEYDOWN
        && message.wParam == VK_RETURN as usize
        && GetDlgCtrlID(message.hwnd) == CUTIN_CONTINUE as i32
    {
        SendMessageW(window, WM_COMMAND, CUTIN_CONTINUE, 0);
        return true;
    }
    if message.message == WM_KEYDOWN && GetDlgCtrlID(message.hwnd) == ANSWER as i32 {
        match message.wParam as u16 {
            VK_RETURN => {
                SendMessageW(window, WM_COMMAND, ATTACK, 0);
                return true;
            }
            VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN if GetKeyState(VK_CONTROL as i32) >= 0 => {
                SendMessageW(window, MOVE_CURSOR, message.wParam, 0);
                return true;
            }
            _ => {}
        }
    }
    IsDialogMessageW(window, message) != 0
}
