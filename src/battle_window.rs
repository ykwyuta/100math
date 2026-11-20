use crate::windows::wide;
use hundred_math::battle::{Battle, Outcome, OPPONENTS};
use hundred_math::{Operation, CELLS, SIDE};
use std::ptr::{null, null_mut};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemServices::SS_CENTER;
use windows_sys::Win32::UI::Controls::{EM_SETLIMITTEXT, EM_SETSEL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, SetActiveWindow, SetFocus, VK_RETURN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ATTACK: usize = 1;
const PRACTICE: usize = 2;
const START: usize = 3;
const OPPONENT: usize = 4;
const OPERATION: usize = 5;
const ANSWER: usize = 6;
const TIMER: usize = 1;
const PORTRAITS: &[u8; 12 * 72 * 96 * 4] = include_bytes!("../assets/opponents.bgra");

struct Round {
    game: Battle,
    started: Instant,
    solved: [Option<i32>; CELLS],
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
    hp: HWND,
    stats: HWND,
    question: HWND,
    note: HWND,
    start: HWND,
    font: HFONT,
    opponent: usize,
    round: Option<Round>,
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
        let control = |class: &str, label: &str, style, id, (x, y, w, h)| {
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
        let hp = control("STATIC", "", 0, 0, (121, 77, 461, 24));
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
        let attack = control("BUTTON", "攻撃", WS_TABSTOP, ATTACK, (409, 131, 79, 30));
        let note = control("STATIC", "", 0, 0, (121, 164, 461, 25));
        control(
            "STATIC",
            "正解が続くほど攻撃が強くなります。",
            SS_CENTER,
            0,
            (18, 535, 564, 25),
        );
        let mut state = Box::new(Self {
            opponent_picker,
            operation_picker,
            answer,
            attack,
            hp,
            stats,
            question,
            note,
            start,
            font,
            opponent: 0,
            round: None,
        });
        state.reset(hwnd);
        state
    }

    unsafe fn enable_answer(&self, enabled: bool) {
        EnableWindow(self.answer, enabled.into());
        EnableWindow(self.attack, enabled.into());
    }

    unsafe fn reset(&mut self, hwnd: HWND) {
        KillTimer(hwnd, TIMER);
        self.round = None;
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
            solved: [None; CELLS],
        });
        self.enable_answer(true);
        set_text(self.start, "対戦をやり直す");
        set_text(self.answer, "");
        set_text(self.note, "答えを入力して攻撃しよう！");
        SetFocus(self.answer);
        SetTimer(hwnd, TIMER, 150, None);
        self.refresh();
        InvalidateRect(hwnd, null(), 1);
    }

    unsafe fn refresh(&self) {
        let (player, opponent, maximum, stats) = match &self.round {
            Some(round) => {
                let game = &round.game;
                let stats = if game.outcome == Outcome::Playing {
                    format!(
                        "{} 連鎖     相手の攻撃まで {} 秒",
                        game.combo,
                        game.remaining(round.elapsed()).div_ceil(1000)
                    )
                } else {
                    "対戦終了".to_owned()
                };
                (
                    game.player_hp,
                    game.opponent_hp,
                    game.opponent_max_hp,
                    stats,
                )
            }
            None => {
                let hp = 22 + 3 * self.opponent as u32;
                (36, hp, hp, "0 連鎖".to_owned())
            }
        };
        set_text(
            self.hp,
            &format!(
                "あなた {player} / 36     {} {opponent} / {maximum}",
                OPPONENTS[self.opponent]
            ),
        );
        set_text(self.stats, &stats);
        if let Some(round) = &self.round {
            let (row, col) = round.game.question();
            set_text(
                self.question,
                &format!("{row} {} {col} = ?", round.game.operation.symbol()),
            );
        }
    }

    unsafe fn finish_or_refresh(&self, hwnd: HWND, old_attacks: u32) {
        self.refresh();
        let Some(round) = &self.round else {
            return;
        };
        match round.game.outcome {
            Outcome::Playing if round.game.attacks != old_attacks => {
                set_text(self.note, &format!("{}の攻撃！", OPPONENTS[self.opponent]));
            }
            Outcome::Playing => {}
            outcome => {
                KillTimer(hwnd, TIMER);
                self.enable_answer(false);
                set_text(self.start, "もう一度");
                set_text(
                    self.note,
                    if outcome == Outcome::Win {
                        "勝ち！ よくできました。"
                    } else {
                        "相手の勝ち。もう一度チャレンジ！"
                    },
                );
                SetFocus(self.start);
                InvalidateRect(hwnd, null(), 1);
            }
        }
    }

    unsafe fn tick(&mut self, hwnd: HWND) {
        let Some(round) = &mut self.round else {
            return;
        };
        if round.game.outcome != Outcome::Playing {
            return;
        }
        let old_attacks = round.game.attacks;
        round.game.tick(round.elapsed());
        self.finish_or_refresh(hwnd, old_attacks);
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
        let old_attacks = round.game.attacks;
        let cell = round.game.cell;
        match round.game.submit(value, round.elapsed()) {
            Some(true) => {
                round.solved[cell] = Some(value);
                set_text(
                    self.note,
                    &format!("正解！ {}連鎖で攻撃！", round.game.combo),
                );
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
        self.finish_or_refresh(hwnd, old_attacks);
    }

    unsafe fn paint(&self, hwnd: HWND) {
        let mut paint: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 72;
        info.bmiHeader.biHeight = -96;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let at = self.opponent * 72 * 96 * 4;
        StretchDIBits(
            dc,
            23,
            80,
            81,
            108,
            0,
            0,
            72,
            96,
            PORTRAITS[at..].as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );

        if let Some(round) = &self.round {
            let previous_font = SelectObject(dc, self.font);
            SetBkMode(dc, TRANSPARENT as i32);
            let header = CreateSolidBrush(0x00e7efe4);
            let current = CreateSolidBrush(0x00bff0ff);
            let solved = CreateSolidBrush(0x00e0ebd7);
            for row in 0..=SIDE {
                for col in 0..=SIDE {
                    let x = 19 + 51 * col as i32;
                    let y = 198 + 30 * row as i32;
                    let mut rect = box_rect(x, y, x + 50, y + 29);
                    let (value, brush) = match (row, col) {
                        (0, 0) => (round.game.operation.symbol().to_owned(), header),
                        (0, col) => (round.game.numbers[SIDE + col - 1].to_string(), header),
                        (row, 0) => (round.game.numbers[row - 1].to_string(), header),
                        (row, col) => {
                            let index = (row - 1) * SIDE + col - 1;
                            let brush = if round.game.outcome == Outcome::Playing
                                && index == round.game.cell
                            {
                                current
                            } else if round.solved[index].is_some() {
                                solved
                            } else {
                                GetSysColorBrush(COLOR_WINDOW)
                            };
                            (
                                round.solved[index].map_or_else(String::new, |n| n.to_string()),
                                brush,
                            )
                        }
                    };
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
            for brush in [header, current, solved] {
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
            DeleteObject(Box::from_raw(state).font);
        }
        WM_PAINT if !state.is_null() => {
            (*state).paint(hwnd);
            return 0;
        }
        WM_COMMAND if !state.is_null() => {
            let id = w & 0xffff;
            let event = (w >> 16) as u32;
            if id == PRACTICE && event == BN_CLICKED {
                SendMessageW(hwnd, WM_CLOSE, 0, 0);
                return 0;
            }
            // Control updates can synchronously send another window message.
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            match (id, event) {
                (OPPONENT | OPERATION, CBN_SELCHANGE) => (*state).reset(hwnd),
                (START, BN_CLICKED) => (*state).start(hwnd),
                (ATTACK, BN_CLICKED) => (*state).submit(hwnd),
                _ => {}
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
        wide("100マス計算 対戦").as_ptr(),
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
        SetFocus(GetDlgItem(window, OPPONENT as i32));
    }
}

pub unsafe fn translate(window: HWND, message: &MSG) -> bool {
    if message.message == WM_KEYDOWN
        && message.wParam == VK_RETURN as usize
        && GetDlgCtrlID(message.hwnd) == ANSWER as i32
    {
        SendMessageW(window, WM_COMMAND, ATTACK, 0);
        return true;
    }
    IsDialogMessageW(window, message) != 0
}
