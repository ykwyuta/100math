use crate::windows::{game_icon, wide};
use hundred_math::collection::{unlocked_coins, CATALOG, COIN_COUNT};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetActiveWindow, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const LIST: usize = 80;
const CLOSE: usize = 81;
const COIN_SIDE: usize = 64;
const COIN_BYTES: usize = COIN_SIDE * COIN_SIDE * 3;
const COINS: &[u8; COIN_BYTES * COIN_COUNT] = include_bytes!("../assets/coins.bgr");

pub fn load_points() -> u32 {
    let mut points = 0;
    let mut size = 4;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(r"Software\HundredMath").as_ptr(),
            wide("PointsV1").as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut points as *mut u32).cast(),
            &mut size,
        )
    };
    if result == ERROR_SUCCESS {
        points
    } else {
        0
    }
}

pub fn save_points(points: u32) -> bool {
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            wide(r"Software\HundredMath").as_ptr(),
            wide("PointsV1").as_ptr(),
            REG_DWORD,
            (&points as *const u32).cast(),
            4,
        ) == ERROR_SUCCESS
    }
}

struct Entry {
    era: &'static str,
    name: &'static str,
    dates: &'static str,
    description: &'static str,
}

fn entries() -> impl Iterator<Item = Entry> {
    let mut era = "";
    CATALOG.lines().filter_map(move |line| {
        if let Some(heading) = line.strip_prefix("# ") {
            era = heading;
            return None;
        }
        let mut fields = line.split('\t');
        Some(Entry {
            era,
            name: fields.next()?,
            dates: fields.next()?,
            description: fields.next()?,
        })
    })
}

pub fn coin_name(index: usize) -> Option<&'static str> {
    entries().nth(index).map(|entry| entry.name)
}

struct Collection {
    entries: Vec<Entry>,
    unlocked: usize,
    index: usize,
    list: HWND,
    era: HWND,
    title: HWND,
    dates: HWND,
    description: HWND,
    font: HFONT,
    source: HWND,
}

impl Collection {
    unsafe fn new(hwnd: HWND, points: u32) -> Box<Self> {
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
            let control = CreateWindowExW(
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
            SendMessageW(control, WM_SETFONT, font as WPARAM, 1);
            control
        };

        let unlocked = unlocked_coins(points);
        let entries: Vec<_> = entries().collect();
        let summary = format!("{points} ポイント ／ {unlocked} / {} 枚", entries.len());
        control("STATIC", &summary, 0, 0, (16, 13, 410, 28));
        let list = control(
            "LISTBOX",
            "",
            WS_TABSTOP | WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
            LIST,
            (16, 50, 278, 379),
        );
        let era = control("STATIC", "", 0, 0, (310, 53, 310, 24));
        let title = control("STATIC", "", 0, 0, (310, 83, 305, 48));
        let dates = control("STATIC", "", 0, 0, (310, 132, 310, 25));
        let description = control("STATIC", "", 0, 0, (310, 307, 303, 109));
        let source = control("STATIC", "", 0, 0, (310, 270, 303, 26));
        control("BUTTON", "閉じる", WS_TABSTOP, CLOSE, (500, 431, 112, 30));

        for (index, entry) in entries.iter().enumerate() {
            let mark = if index < unlocked { "●" } else { "○" };
            let label = format!("{mark} {:03}  {}", index + 1, entry.name);
            SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as LPARAM);
        }
        let index = unlocked.saturating_sub(1);
        SendMessageW(list, LB_SETCURSEL, index, 0);
        let state = Box::new(Self {
            entries,
            unlocked,
            index,
            list,
            era,
            title,
            dates,
            description,
            font,
            source,
        });
        state.refresh(hwnd);
        state
    }

    unsafe fn refresh(&self, hwnd: HWND) {
        if let Some(entry) = self.entries.get(self.index) {
            SetWindowTextW(self.era, wide(entry.era).as_ptr());
            SetWindowTextW(self.title, wide(entry.name).as_ptr());
            SetWindowTextW(self.dates, wide(entry.dates).as_ptr());
            SetWindowTextW(self.description, wide(entry.description).as_ptr());
            let status = if self.index < self.unlocked {
                "獲得済み"
            } else {
                "未獲得"
            };
            SetWindowTextW(self.source, wide(status).as_ptr());
        }
        InvalidateRect(hwnd, null(), 1);
    }

    unsafe fn paint(&self, hwnd: HWND) {
        let mut paint: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        if self.index < self.unlocked {
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = COIN_SIDE as i32;
            info.bmiHeader.biHeight = -(COIN_SIDE as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 24;
            info.bmiHeader.biCompression = BI_RGB;
            let start = self.index * COIN_BYTES;
            SetStretchBltMode(dc, HALFTONE);
            StretchDIBits(
                dc,
                310,
                157,
                108,
                108,
                0,
                0,
                COIN_SIDE as i32,
                COIN_SIDE as i32,
                COINS[start..].as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        } else {
            Ellipse(dc, 312, 158, 416, 262);
            let mut rect = RECT {
                left: 312,
                top: 158,
                right: 416,
                bottom: 262,
            };
            let previous_font = SelectObject(dc, self.font);
            SetBkMode(dc, TRANSPARENT as i32);
            DrawTextW(
                dc,
                wide("？").as_ptr(),
                -1,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            SelectObject(dc, previous_font);
        }
        EndPaint(hwnd, &paint);
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Collection;
    match message {
        WM_CREATE => {
            let create = &*(l as *const CREATESTRUCTW);
            let points = create.lpCreateParams as usize as u32;
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                Box::into_raw(Collection::new(hwnd, points)) as isize,
            );
            return 0;
        }
        WM_CLOSE => {
            let owner = GetWindow(hwnd, GW_OWNER);
            EnableWindow(owner, 1);
            DestroyWindow(hwnd);
            SetActiveWindow(owner);
            SetFocus(GetDlgItem(owner, super::battle_window::COLLECTION as i32));
            return 0;
        }
        WM_NCDESTROY if !state.is_null() => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DeleteObject(Box::from_raw(state).font);
        }
        WM_COMMAND if (w & 0xffff) == CLOSE || (w & 0xffff) == IDCANCEL as usize => {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
            return 0;
        }
        WM_COMMAND
            if !state.is_null() && (w & 0xffff) == LIST && (w >> 16) == LBN_SELCHANGE as usize =>
        {
            let index = SendMessageW((*state).list, LB_GETCURSEL, 0, 0);
            if index >= 0 {
                (*state).index = index as usize;
                (*state).refresh(hwnd);
            }
            return 0;
        }
        WM_PAINT if !state.is_null() => {
            (*state).paint(hwnd);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, w, l)
}

pub unsafe fn open(owner: HWND, points: u32) {
    let class = wide("HundredMathCollection");
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
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 630,
        bottom: 475,
    };
    AdjustWindowRectEx(&mut rect, style, 0, WS_EX_CONTROLPARENT);
    let mut anchor = std::mem::zeroed();
    GetWindowRect(owner, &mut anchor);
    let window = CreateWindowExW(
        WS_EX_CONTROLPARENT,
        class.as_ptr(),
        wide("数学者コイン図鑑").as_ptr(),
        style,
        anchor.left,
        anchor.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        owner,
        null_mut(),
        instance,
        points as usize as *mut _,
    );
    if !window.is_null() {
        EnableWindow(owner, 0);
        ShowWindow(window, SW_SHOW);
        SetFocus(GetDlgItem(window, LIST as i32));
    }
}
