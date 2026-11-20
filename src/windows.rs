use hundred_math::{axes, mark, Mark, Operation, CELLS, SIDE};
use std::ptr::{null, null_mut};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemServices::SS_CENTER;
use windows_sys::Win32::UI::Controls::{EM_SETLIMITTEXT, EM_SETREADONLY, EM_SETSEL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, IsWindowEnabled, SetFocus, VK_CONTROL, VK_DOWN, VK_LEFT, VK_RETURN,
    VK_RIGHT, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const OPEN_BATTLE: usize = 104;
const NEW: usize = 101;
const CHECK: usize = 102;
const OPERATION: usize = 103;
const FIRST_CELL: usize = 200;
const TIMER: usize = 1;
const STEP_X: i32 = 50;
const STEP_Y: i32 = 33;

pub unsafe fn game_icon(instance: HINSTANCE) -> HICON {
    // Resource 101 is the multi-size icon in assets/game.rc.
    LoadIconW(instance, 101usize as *const u16)
}

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

struct Window {
    edits: [HWND; CELLS],
    row_labels: [HWND; SIDE],
    column_labels: [HWND; SIDE],
    corner: HWND,
    operation_picker: HWND,
    metrics: HWND,
    check: HWND,
    result: HWND,
    font: HFONT,
    numbers: [i32; SIDE * 2],
    operation: Operation,
    started: Option<Instant>,
    graded: bool,
}

impl Window {
    unsafe fn new(hwnd: HWND) -> Box<Self> {
        let font = CreateFontW(
            -16,
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
        let operation_picker = control(
            "COMBOBOX",
            "",
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
            OPERATION,
            (20, 46, 138, 140),
        );
        for label in ["＋ たし算", "− ひき算", "× かけ算"] {
            SendMessageW(
                operation_picker,
                CB_ADDSTRING,
                0,
                wide(label).as_ptr() as LPARAM,
            );
        }
        SendMessageW(operation_picker, CB_SETCURSEL, 0, 0);
        control(
            "STATIC",
            "左の数と上の数を使って、マスを埋めよう。",
            0,
            0,
            (20, 14, 545, 26),
        );
        let metrics = control("STATIC", "", 0, 0, (180, 49, 375, 26));
        let corner = control("STATIC", "", SS_CENTER, 0, (20, 86, 48, 28));
        let column_labels = std::array::from_fn(|column| {
            control(
                "STATIC",
                "",
                SS_CENTER,
                0,
                (70 + column as i32 * STEP_X, 86, 48, 28),
            )
        });
        let row_labels = std::array::from_fn(|row| {
            control(
                "STATIC",
                "",
                SS_CENTER,
                0,
                (20, 121 + row as i32 * STEP_Y, 48, 28),
            )
        });
        let edits = std::array::from_fn(|index| {
            let row = (index / SIDE) as i32;
            let col = (index % SIDE) as i32;
            let edit = control(
                "EDIT",
                "",
                WS_BORDER | WS_TABSTOP | ES_CENTER as u32 | ES_AUTOHSCROLL as u32,
                FIRST_CELL + index,
                (70 + col * STEP_X, 117 + row * STEP_Y, 48, 30),
            );
            SendMessageW(edit, EM_SETLIMITTEXT, 3, 0);
            edit
        });
        control("BUTTON", "新しい問題", WS_TABSTOP, NEW, (20, 455, 120, 34));
        let check = control("BUTTON", "採点する", WS_TABSTOP, CHECK, (150, 455, 120, 34));
        control(
            "BUTTON",
            "対戦する",
            WS_TABSTOP,
            OPEN_BATTLE,
            (435, 455, 120, 34),
        );
        let result = control("STATIC", "", 0, 0, (20, 500, 545, 28));
        control(
            "STATIC",
            "Enter・Tab・矢印キーで移動。ひき算は「左 − 上」です。",
            0,
            0,
            (20, 536, 545, 28),
        );
        Box::new(Self {
            edits,
            row_labels,
            column_labels,
            corner,
            operation_picker,
            metrics,
            check,
            result,
            font,
            numbers: [0; SIDE * 2],
            operation: Operation::Add,
            started: None,
            graded: false,
        })
    }

    unsafe fn new_board(&mut self, hwnd: HWND, operation: Operation) {
        KillTimer(hwnd, TIMER);
        self.started = None;
        self.graded = false;
        self.operation = operation;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut seed = stamp as u32 ^ (stamp >> 32) as u32;
        while axes(seed) == self.numbers {
            seed = seed.wrapping_add(1);
        }
        self.numbers = axes(seed);
        SetWindowTextW(self.corner, wide(operation.symbol()).as_ptr());
        for i in 0..SIDE {
            SetWindowTextW(
                self.row_labels[i],
                wide(&self.numbers[i].to_string()).as_ptr(),
            );
            SetWindowTextW(
                self.column_labels[i],
                wide(&self.numbers[SIDE + i].to_string()).as_ptr(),
            );
        }
        for edit in self.edits {
            SetWindowTextW(edit, wide("").as_ptr());
            SendMessageW(edit, EM_SETREADONLY, 0, 0);
        }
        EnableWindow(self.check, 0);
        SetWindowTextW(
            self.result,
            wide("答えを入力すると、計測が始まります。").as_ptr(),
        );
        self.update_metrics();
    }

    unsafe fn answer(edit: HWND) -> Option<i32> {
        let mut text = [0; 4];
        let size = GetWindowTextW(edit, text.as_mut_ptr(), text.len() as i32) as usize;
        String::from_utf16_lossy(&text[..size])
            .chars()
            .map(|ch| match ch {
                '０'..='９' => char::from(b'0' + (ch as u32 - '０' as u32) as u8),
                '−' | '－' => '-',
                _ => ch,
            })
            .collect::<String>()
            .parse()
            .ok()
    }

    unsafe fn update_metrics(&self) -> usize {
        let filled = self
            .edits
            .iter()
            .filter(|&&edit| Self::answer(edit).is_some())
            .count();
        let seconds = self.started.map_or(0, |t| t.elapsed().as_secs());
        let text = format!(
            "入力済み  {filled} / {CELLS}       タイム  {:02}:{:02}",
            seconds / 60,
            seconds % 60
        );
        SetWindowTextW(self.metrics, wide(&text).as_ptr());
        filled
    }

    unsafe fn on_edit(&mut self, hwnd: HWND) {
        let filled = self.update_metrics();
        EnableWindow(self.check, (filled > 0).into());
        if filled > 0 && self.started.is_none() {
            self.started = Some(Instant::now());
            SetTimer(hwnd, TIMER, 250, None);
            SetWindowTextW(
                self.result,
                wide("入力ができたら「採点する」を押してください。").as_ptr(),
            );
        }
    }

    unsafe fn grade(&mut self, hwnd: HWND) {
        KillTimer(hwnd, TIMER);
        self.update_metrics();
        self.graded = true;
        let mut correct = 0;
        let mut incorrect = 0;
        for (index, edit) in self.edits.iter().copied().enumerate() {
            let row = self.numbers[index / SIDE];
            let col = self.numbers[SIDE + index % SIDE];
            match mark(self.operation, row, col, Self::answer(edit)) {
                Mark::Correct => correct += 1,
                Mark::Incorrect => incorrect += 1,
                Mark::Blank => {}
            }
            SendMessageW(edit, EM_SETREADONLY, 1, 0);
        }
        let blank = CELLS - correct - incorrect;
        let text = format!("正解 {correct}問 ／ 不正解 {incorrect}問 ／ 未入力 {blank}問");
        SetWindowTextW(self.result, wide(&text).as_ptr());
        SetFocus(GetDlgItem(hwnd, NEW as i32));
        EnableWindow(self.check, 0);
    }

    unsafe fn key(&self, hwnd: HWND, msg: &MSG) -> bool {
        let Some(index) = self.edits.iter().position(|&edit| edit == msg.hwnd) else {
            return false;
        };
        if GetKeyState(VK_CONTROL as i32) < 0 {
            return false;
        }
        let next = match msg.wParam as u16 {
            VK_RETURN if index == CELLS - 1 => {
                let target = if IsWindowEnabled(self.check) != 0 {
                    self.check
                } else {
                    GetDlgItem(hwnd, NEW as i32)
                };
                SetFocus(target);
                return true;
            }
            VK_RETURN => index + 1,
            VK_LEFT if index % SIDE > 0 => index - 1,
            VK_RIGHT if index % SIDE < SIDE - 1 => index + 1,
            VK_UP if index >= SIDE => index - SIDE,
            VK_DOWN if index + SIDE < CELLS => index + SIDE,
            VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN => return true,
            _ => return false,
        };
        SetFocus(self.edits[next]);
        SendMessageW(self.edits[next], EM_SETSEL, 0, -1);
        true
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Window;
    match message {
        WM_CREATE => {
            let mut window = Window::new(hwnd);
            window.new_board(hwnd, Operation::Add);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(window) as isize);
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        WM_NCDESTROY if !state.is_null() => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let window = Box::from_raw(state);
            DeleteObject(window.font);
        }
        WM_COMMAND if !state.is_null() => {
            let id = w & 0xffff;
            let notification = (w >> 16) as u32;

            if id == OPEN_BATTLE && notification == BN_CLICKED {
                crate::battle_window::open(hwnd);
                return 0;
            }

            // Updating a child can synchronously reenter this procedure.
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            match id {
                NEW if notification == BN_CLICKED => {
                    let operation = (*state).operation;
                    (*state).new_board(hwnd, operation);
                    SetFocus((*state).edits[0]);
                }
                CHECK if notification == BN_CLICKED && !(*state).graded => {
                    (*state).grade(hwnd);
                }
                OPERATION if notification == CBN_SELCHANGE => {
                    let index = SendMessageW((*state).operation_picker, CB_GETCURSEL, 0, 0);
                    (*state).new_board(hwnd, Operation::from_index(index as u32));
                }
                FIRST_CELL..
                    if id < FIRST_CELL + CELLS && notification == EN_CHANGE && !(*state).graded =>
                {
                    (*state).on_edit(hwnd);
                }
                _ => {}
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            if id == CHECK && notification == BN_CLICKED {
                RedrawWindow(hwnd, null(), null_mut(), RDW_INVALIDATE | RDW_ALLCHILDREN);
            }
            return 0;
        }
        WM_TIMER if w == TIMER && !state.is_null() && !(*state).graded => {
            (*state).update_metrics();
            return 0;
        }
        WM_CTLCOLORSTATIC if !state.is_null() && (*state).graded => {
            if let Some(index) = (*state).edits.iter().position(|&edit| edit == l as HWND) {
                let row = (*state).numbers[index / SIDE];
                let col = (*state).numbers[SIDE + index % SIDE];
                let color = match mark((*state).operation, row, col, Window::answer(l as HWND)) {
                    Mark::Correct => 0x00386415,
                    Mark::Incorrect => 0x002020a0,
                    Mark::Blank => GetSysColor(COLOR_WINDOWTEXT),
                };
                SetTextColor(w as HDC, color);
                SetBkColor(w as HDC, GetSysColor(COLOR_WINDOW));
                return GetSysColorBrush(COLOR_WINDOW) as LRESULT;
            }
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, w, l)
}

pub fn run() {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = wide("HundredMath");
        let definition = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hIcon: game_icon(instance),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..std::mem::zeroed()
        };
        let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 590,
            bottom: 580,
        };
        AdjustWindowRectEx(&mut rect, style, 0, WS_EX_CONTROLPARENT);
        let hwnd = if RegisterClassW(&definition) != 0 {
            CreateWindowExW(
                WS_EX_CONTROLPARENT,
                class.as_ptr(),
                wide("100マス計算").as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                null_mut(),
                null_mut(),
                instance,
                null(),
            )
        } else {
            null_mut()
        };
        if hwnd.is_null() {
            MessageBoxW(
                null_mut(),
                wide("ウィンドウを作成できませんでした。").as_ptr(),
                wide("100マス計算").as_ptr(),
                MB_OK | MB_ICONERROR,
            );
            return;
        }
        ShowWindow(hwnd, SW_SHOW);
        SetFocus(GetDlgItem(hwnd, FIRST_CELL as i32));
        crate::battle_window::open(hwnd);
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            let root = GetAncestor(message.hwnd, GA_ROOT);
            if !root.is_null() && root != hwnd && GetAncestor(root, GA_ROOTOWNER) == hwnd {
                if !crate::battle_window::translate(root, &message) {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                continue;
            }
            if message.message == WM_KEYDOWN {
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Window;
                if !state.is_null() && (*state).key(hwnd, &message) {
                    continue;
                }
            }
            if IsDialogMessageW(hwnd, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
