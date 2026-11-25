use crate::models::{
    AutoImportSummary, DeckCard, GameWindowStatus, ImporterCalibration, SearchCardResult,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PreparedImportCard {
    pub name: String,
    pub search_query: String,
    pub amount: u32,
    pub disambiguated: bool,
}

/// Precomputes a collision-free Master Duel search query for every card in the deck.
/// Because Master Duel's search bar matches ALL space-separated keywords across (Name + Card Text),
/// whenever a card name like "Red Dragon Archfiend" collides with support cards mentioning it,
/// we greedily append 1–3 distinctive words from its own effect text to isolate it to 1 card (Slot #1)!
pub fn prepare_smart_import_cards(
    cards: &[DeckCard],
    catalog: &[SearchCardResult],
) -> Vec<PreparedImportCard> {
    if catalog.is_empty() {
        return cards
            .iter()
            .map(|c| PreparedImportCard {
                name: c.name.clone(),
                search_query: sanitize_name_for_md_search(&c.name),
                amount: c.amount,
                disambiguated: false,
            })
            .collect();
    }

    // Build lowercase searchable haystacks once (~2ms for 14k cards)
    let corpus: Vec<(&SearchCardResult, String)> = catalog
        .iter()
        .map(|c| {
            let desc = c.description.as_deref().unwrap_or("");
            let hay = format!("{}\n{}", c.name, desc).to_lowercase();
            (c, hay)
        })
        .collect();

    cards
        .iter()
        .map(|deck_card| {
            let clean_base = sanitize_name_for_md_search(&deck_card.name);
            let target_opt = corpus
                .iter()
                .find(|(c, _)| c.id == deck_card.id)
                .or_else(|| {
                    let nl = deck_card.name.to_lowercase();
                    corpus.iter().find(|(c, _)| c.name.to_lowercase() == nl)
                });

            let Some((target, target_hay)) = target_opt else {
                return PreparedImportCard {
                    name: deck_card.name.clone(),
                    search_query: clean_base,
                    amount: deck_card.amount,
                    disambiguated: false,
                };
            };

            let mut query_tokens: Vec<String> = clean_base
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            // Filter corpus by all base name tokens
            let mut remaining: Vec<usize> = (0..corpus.len())
                .filter(|&idx| {
                    let hay = &corpus[idx].1;
                    query_tokens
                        .iter()
                        .all(|tk| hay.contains(&tk.to_lowercase()))
                })
                .collect();

            if remaining.len() <= 1 {
                return PreparedImportCard {
                    name: deck_card.name.clone(),
                    search_query: query_tokens.join(" "),
                    amount: deck_card.amount,
                    disambiguated: false,
                };
            }

            // Card name collides with other cards (e.g. "Red Dragon Archfiend" -> 38 cards)!
            // Extract candidate alphanumeric words from its own description to isolate it to 1 card.
            let mut seen_words: HashSet<String> = query_tokens
                .iter()
                .map(|s| s.to_lowercase())
                .collect();
            let mut cand_words: Vec<String> = Vec::new();

            if let Some(ref desc) = target.description {
                for raw_w in desc.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-') {
                    let w = raw_w.trim_matches('-');
                    if w.len() >= 3 && w.len() <= 18 {
                        let wl = w.to_lowercase();
                        if target_hay.contains(&wl) && seen_words.insert(wl) {
                            cand_words.push(w.to_string());
                        }
                    }
                }
            }

            let mut added_any = false;
            for _step in 0..5 {
                if remaining.len() <= 1 {
                    break;
                }
                let cur_len: usize =
                    query_tokens.iter().map(|s| s.len()).sum::<usize>() + query_tokens.len();

                let mut best_word: Option<String> = None;
                let mut best_count: usize = remaining.len();
                let mut best_word_len: usize = 0;

                for dw in &cand_words {
                    if cur_len + dw.len() > 46 {
                        continue;
                    }
                    let dwl = dw.to_lowercase();
                    if query_tokens.iter().any(|q| q.eq_ignore_ascii_case(dw)) {
                        continue;
                    }

                    let mut match_count = 0usize;
                    let mut contains_target = false;
                    for &idx in &remaining {
                        if corpus[idx].1.contains(&dwl) {
                            match_count += 1;
                            if corpus[idx].0.id == target.id {
                                contains_target = true;
                            }
                        }
                    }

                    if !contains_target || match_count == 0 {
                        continue;
                    }

                    // Prefer strictly fewer matches; on ties, prefer cleaner longer words (up to 12 chars)
                    let eff_len = dw.len().min(12);
                    let is_better = match_count < best_count
                        || (match_count == best_count && eff_len > best_word_len);

                    if is_better {
                        best_count = match_count;
                        best_word_len = eff_len;
                        best_word = Some(dw.clone());
                    }
                }

                if let Some(chosen) = best_word {
                    let cl = chosen.to_lowercase();
                    remaining.retain(|&idx| corpus[idx].1.contains(&cl));
                    query_tokens.push(chosen);
                    added_any = true;
                } else {
                    break;
                }
            }

            PreparedImportCard {
                name: deck_card.name.clone(),
                search_query: query_tokens.join(" "),
                amount: deck_card.amount,
                disambiguated: added_any,
            }
        })
        .collect()
}

fn sanitize_name_for_md_search(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            '"' | '(' | ')' => ' ',
            _ => ch,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
mod win_input {
    use crate::models::{
        GameWindowStatus, ImporterCalibration, UNOWNED_TOGGLE_RX, UNOWNED_TOGGLE_RY,
    };
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC,
        DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
        KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VIRTUAL_KEY, VK_A, VK_BACK,
        VK_CONTROL, VK_ESCAPE, VK_RETURN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetClientRect, GetCursorPos, GetForegroundWindow,
        GetWindowTextW, IsIconic, IsWindowVisible, SetCursorPos, SetForegroundWindow, ShowWindow,
        SW_RESTORE,
    };

    struct FindMdState {
        best_hwnd: Option<HWND>,
        best_score: u8,
        best_title: String,
    }

    unsafe extern "system" fn enum_md_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut FindMdState);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut title_buf = [0u16; 256];
        let tlen = GetWindowTextW(hwnd, &mut title_buf);
        if tlen <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&title_buf[..tlen as usize]);
        let title_lower = title.trim().to_lowercase();
        if title_lower.contains("duelcraft") {
            return BOOL(1);
        }

        let mut class_buf = [0u16; 128];
        let clen = GetClassNameW(hwnd, &mut class_buf);
        let class_name = if clen > 0 {
            String::from_utf16_lossy(&class_buf[..clen as usize])
        } else {
            String::new()
        };

        let is_unity = class_name == "UnityWndClass";
        let score = if is_unity
            && (title_lower == "masterduel" || title_lower.contains("master duel"))
        {
            2
        } else if title_lower == "masterduel" {
            1
        } else {
            0
        };

        if score > state.best_score {
            state.best_score = score;
            state.best_hwnd = Some(hwnd);
            state.best_title = title;
            if score == 2 {
                return BOOL(0);
            }
        }
        BOOL(1)
    }

    fn find_master_duel_window() -> Option<(HWND, String)> {
        let mut state = FindMdState {
            best_hwnd: None,
            best_score: 0,
            best_title: String::new(),
        };
        unsafe {
            let _ = EnumWindows(
                Some(enum_md_proc),
                LPARAM(&mut state as *mut FindMdState as isize),
            );
        }
        state.best_hwnd.map(|h| (h, state.best_title))
    }

    /// Computes Unity's active 16:9 scaled UI viewport inside the window's client area.
    pub fn compute_16_9_viewport(cx: i32, cy: i32, cw: i32, ch: i32) -> (f64, f64, f64, f64) {
        let fw = (cw.max(1)) as f64;
        let fh = (ch.max(1)) as f64;
        let target = 16.0 / 9.0;
        let aspect = fw / fh;

        if (aspect - target).abs() < 0.015 {
            (cx as f64, cy as f64, fw, fh)
        } else if aspect > target {
            // Ultrawide: pillarboxed horizontally
            let vw = fh * target;
            let vx = (cx as f64) + (fw - vw) * 0.5;
            (vx, cy as f64, vw, fh)
        } else {
            // Taller than 16:9 (e.g. 16:10): letterboxed vertically
            let vh = fw / target;
            let vy = (cy as f64) + (fh - vh) * 0.5;
            (cx as f64, vy, fw, vh)
        }
    }

    pub fn query_window_status(cal: &ImporterCalibration) -> GameWindowStatus {
        let Some((hwnd, title)) = find_master_duel_window() else {
            return GameWindowStatus {
                found: false,
                minimized: false,
                title: String::new(),
                client_x: 0,
                client_y: 0,
                client_width: 0,
                client_height: 0,
                search_x: 0,
                search_y: 0,
                first_card_x: 0,
                first_card_y: 0,
            };
        };

        unsafe {
            let minimized = IsIconic(hwnd).as_bool();
            if minimized {
                return GameWindowStatus {
                    found: true,
                    minimized: true,
                    title,
                    client_x: 0,
                    client_y: 0,
                    client_width: 0,
                    client_height: 0,
                    search_x: 0,
                    search_y: 0,
                    first_card_x: 0,
                    first_card_y: 0,
                };
            }

            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let cw = rect.right - rect.left;
            let ch = rect.bottom - rect.top;

            let mut origin = POINT { x: 0, y: 0 };
            let _ = ClientToScreen(hwnd, &mut origin);

            let (vx, vy, vw, vh) = compute_16_9_viewport(origin.x, origin.y, cw, ch);
            let search_x = (vx + cal.search_rx * vw).round() as i32;
            let search_y = (vy + cal.search_ry * vh).round() as i32;
            let first_card_x = (vx + cal.first_card_rx * vw).round() as i32;
            let first_card_y = (vy + cal.first_card_ry * vh).round() as i32;

            GameWindowStatus {
                found: true,
                minimized: false,
                title,
                client_x: origin.x,
                client_y: origin.y,
                client_width: cw,
                client_height: ch,
                search_x,
                search_y,
                first_card_x,
                first_card_y,
            }
        }
    }

    pub fn focus_master_duel(cal: &ImporterCalibration) -> Result<GameWindowStatus, String> {
        let Some((hwnd, _)) = find_master_duel_window() else {
            return Err(
                "Janela do Master Duel não encontrada! Abra o Master Duel na tela de Edição de Deck e tente novamente."
                    .to_string(),
            );
        };

        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            let _ = SetForegroundWindow(hwnd);
            std::thread::sleep(std::time::Duration::from_millis(120));

            // If Windows blocked SetForegroundWindow across monitors, click neutral empty top header bar once
            if GetForegroundWindow() != hwnd {
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let mut origin = POINT { x: 0, y: 0 };
                let _ = ClientToScreen(hwnd, &mut origin);
                let (vx, vy, vw, vh) = compute_16_9_viewport(
                    origin.x,
                    origin.y,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                );
                let neutral_x = (vx + 0.50 * vw).round() as i32;
                let neutral_y = (vy + 0.04 * vh).round() as i32;
                move_and_left_click(neutral_x, neutral_y);
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        }

        let status = query_window_status(cal);
        if status.client_width < 400 || status.client_height < 300 {
            return Err("Janela do Master Duel está com tamanho inválido ou minimizada.".to_string());
        }
        Ok(status)
    }

    pub fn is_master_duel_foreground() -> bool {
        let Some((hwnd, _)) = find_master_duel_window() else {
            return false;
        };
        unsafe { GetForegroundWindow() == hwnd }
    }

    /// Checks if the "Show Unowned Cards" pill button (top-right next to search bar) is active (lime-green)
    /// or inactive (black). Returns `Ok(true)` if it was black and we clicked to activate it!
    pub fn ensure_unowned_cards_toggle_active(status: &GameWindowStatus) -> Result<bool, String> {
        let (vx, vy, vw, vh) = compute_16_9_viewport(
            status.client_x,
            status.client_y,
            status.client_width,
            status.client_height,
        );
        let toggle_x = (vx + UNOWNED_TOGGLE_RX * vw).round() as i32;
        let toggle_y = (vy + UNOWNED_TOGGLE_RY * vh).round() as i32;
        let sw = ((vw * 0.038).round() as i32).clamp(20, 120);
        let sh = ((vh * 0.010).round() as i32).clamp(6, 24);

        let is_green = is_region_lime_green(toggle_x - sw / 2, toggle_y - sh / 2, sw, sh)?;
        if is_green {
            Ok(false)
        } else {
            move_and_left_click(toggle_x, toggle_y);
            Ok(true)
        }
    }

    pub fn unowned_toggle_coords(status: &GameWindowStatus) -> (i32, i32) {
        let (vx, vy, vw, vh) = compute_16_9_viewport(
            status.client_x,
            status.client_y,
            status.client_width,
            status.client_height,
        );
        (
            (vx + UNOWNED_TOGGLE_RX * vw).round() as i32,
            (vy + UNOWNED_TOGGLE_RY * vh).round() as i32,
        )
    }

    fn is_region_lime_green(x: i32, y: i32, width: i32, height: i32) -> Result<bool, String> {
        unsafe {
            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return Err("Falha ao obter DC da tela.".to_string());
            }
            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            if mem_dc.is_invalid() {
                ReleaseDC(None, screen_dc);
                return Err("Falha ao criar mem_dc.".to_string());
            }
            let hbmp = CreateCompatibleBitmap(screen_dc, width, height);
            if hbmp.is_invalid() {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
                return Err("Falha ao criar bitmap de leitura.".to_string());
            }

            let old = SelectObject(mem_dc, hbmp.into());
            let blt = BitBlt(mem_dc, 0, 0, width, height, Some(screen_dc), x, y, SRCCOPY);

            let mut bmi = BITMAPINFO::default();
            bmi.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };

            let total_px = (width as usize) * (height as usize);
            let mut pixels = vec![0u8; total_px * 4];
            let rows = GetDIBits(
                mem_dc,
                hbmp,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            SelectObject(mem_dc, old);
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);

            if blt.is_err() || rows == 0 {
                return Err("Falha ao ler pixels do botão de cartas não possuídas.".to_string());
            }

            // BGRA byte order: [B, G, R, A]
            // Master Duel active toggle is bright lime-green (R ~ 100..190, G ~ 125..235, B ~ 0..45)
            // Inactive toggle is black (R < 40, G < 45, B < 50)
            let mut green_px = 0usize;
            for px in pixels.chunks_exact(4) {
                let b = px[0] as i32;
                let g = px[1] as i32;
                let r = px[2] as i32;
                if g >= 105 && r >= 70 && b <= 75 && (g - b) >= 55 {
                    green_px += 1;
                }
            }

            Ok(green_px * 100 >= total_px * 15)
        }
    }

    /// Checks whether Slot #1 at (cx, cy) already has a rendered card (even a darkened 0-owned card)
    /// vs the empty dark-navy background while Master Duel is still searching.
    pub fn is_first_card_slot_populated(cx: i32, cy: i32) -> bool {
        let size = 28i32;
        let x = cx - size / 2;
        let y = cy - size / 2;
        unsafe {
            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return true;
            }
            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            if mem_dc.is_invalid() {
                ReleaseDC(None, screen_dc);
                return true;
            }
            let hbmp = CreateCompatibleBitmap(screen_dc, size, size);
            if hbmp.is_invalid() {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
                return true;
            }

            let old = SelectObject(mem_dc, hbmp.into());
            let blt = BitBlt(mem_dc, 0, 0, size, size, Some(screen_dc), x, y, SRCCOPY);

            let mut bmi = BITMAPINFO::default();
            bmi.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };

            let total_px = (size as usize) * (size as usize);
            let mut pixels = vec![0u8; total_px * 4];
            let rows = GetDIBits(
                mem_dc,
                hbmp,
                0,
                size as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            SelectObject(mem_dc, old);
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);

            if blt.is_err() || rows == 0 {
                return true;
            }

            let mut min_lum = 999i32;
            let mut max_lum = 0i32;
            let mut card_px = 0usize;
            for px in pixels.chunks_exact(4) {
                let b = px[0] as i32;
                let g = px[1] as i32;
                let r = px[2] as i32;
                let lum = r + g + b;
                if lum < min_lum {
                    min_lum = lum;
                }
                if lum > max_lum {
                    max_lum = lum;
                }
                if r > 45 || g > 65 {
                    card_px += 1;
                }
            }

            (max_lum - min_lum) >= 95 || card_px >= 50
        }
    }

    pub fn get_cursor_pos() -> Result<(i32, i32), String> {
        let mut pt = POINT::default();
        unsafe {
            GetCursorPos(&mut pt).map_err(|e| format!("Falha ao ler posição do mouse: {}", e))?;
        }
        Ok((pt.x, pt.y))
    }

    pub fn move_cursor_only(x: i32, y: i32) {
        unsafe {
            let _ = SetCursorPos(x, y);
        }
    }

    pub fn is_escape_pressed() -> bool {
        unsafe { (GetAsyncKeyState(VK_ESCAPE.0 as i32) as u16 & 0x8000) != 0 }
    }

    pub fn move_and_left_click(x: i32, y: i32) {
        unsafe {
            let _ = SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(45));
            let down = [mouse_input(MOUSEEVENTF_LEFTDOWN)];
            SendInput(&down, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(35));
            let up = [mouse_input(MOUSEEVENTF_LEFTUP)];
            SendInput(&up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn right_click_card(x: i32, y: i32, first_hover: bool) {
        unsafe {
            // Tiny cursor jitter + SetCursorPos forces Unity's EventSystem PointerEnter raycast
            // to refresh on the newly rendered card slot before clicking!
            let _ = SetCursorPos(x + 2, y);
            std::thread::sleep(std::time::Duration::from_millis(20));
            let _ = SetCursorPos(x, y);
            std::thread::sleep(std::time::Duration::from_millis(if first_hover {
                90
            } else {
                35
            }));

            let down = [mouse_input(MOUSEEVENTF_RIGHTDOWN)];
            SendInput(&down, std::mem::size_of::<INPUT>() as i32);
            // 58ms hold guarantees >= 2 full frames even if Master Duel UI runs at 30 FPS!
            std::thread::sleep(std::time::Duration::from_millis(58));
            let up = [mouse_input(MOUSEEVENTF_RIGHTUP)];
            SendInput(&up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn select_all_and_delete() {
        unsafe {
            // Separate SendInput calls with frame-spanning delays so Unity's Event loop
            // always sees VK_CONTROL held down during VK_A, preventing text overlap!
            let ctrl_down = [vk_input(VK_CONTROL, false)];
            SendInput(&ctrl_down, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(20));

            let a_down = [vk_input(VK_A, false)];
            SendInput(&a_down, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(25));

            let a_up = [vk_input(VK_A, true)];
            SendInput(&a_up, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(15));

            let ctrl_up = [vk_input(VK_CONTROL, true)];
            SendInput(&ctrl_up, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(20));

            let bs_down = [vk_input(VK_BACK, false)];
            SendInput(&bs_down, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(25));

            let bs_up = [vk_input(VK_BACK, true)];
            SendInput(&bs_up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn type_unicode_text(text: &str) {
        let mut inputs: Vec<INPUT> = Vec::with_capacity(text.len() * 2);
        for unit in text.encode_utf16() {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
        if !inputs.is_empty() {
            unsafe {
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
        }
    }

    pub fn press_enter() {
        unsafe {
            let down = [vk_input(VK_RETURN, false)];
            SendInput(&down, std::mem::size_of::<INPUT>() as i32);
            std::thread::sleep(std::time::Duration::from_millis(40));
            let up = [vk_input(VK_RETURN, true)];
            SendInput(&up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn mouse_input(flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if key_up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}

pub fn get_game_window_status(cal: &ImporterCalibration) -> GameWindowStatus {
    #[cfg(windows)]
    {
        win_input::query_window_status(cal)
    }
    #[cfg(not(windows))]
    {
        let _ = cal;
        GameWindowStatus {
            found: false,
            minimized: false,
            title: String::new(),
            client_x: 0,
            client_y: 0,
            client_width: 0,
            client_height: 0,
            search_x: 0,
            search_y: 0,
            first_card_x: 0,
            first_card_y: 0,
        }
    }
}

/// Captures cursor position after delay and converts it to normalized 16:9 ratios inside Master Duel's window,
/// so custom adjustments still follow the window across monitors and resolutions!
pub fn capture_relative_point_after_delay(
    delay_secs: u64,
    cal: &ImporterCalibration,
) -> Result<(f64, f64), String> {
    thread::sleep(Duration::from_secs(delay_secs.clamp(1, 10)));
    #[cfg(windows)]
    {
        let status = win_input::query_window_status(cal);
        if !status.found || status.minimized || status.client_width < 400 {
            return Err(
                "Deixe a janela do Master Duel visível ao capturar o ponto relativo.".to_string(),
            );
        }
        let (mx, my) = win_input::get_cursor_pos()?;
        let (vx, vy, vw, vh) = win_input::compute_16_9_viewport(
            status.client_x,
            status.client_y,
            status.client_width,
            status.client_height,
        );
        let rx = (((mx as f64) - vx) / vw).clamp(0.05, 0.98);
        let ry = (((my as f64) - vy) / vh).clamp(0.05, 0.98);
        Ok(((rx * 10000.0).round() / 10000.0, (ry * 10000.0).round() / 10000.0))
    }
    #[cfg(not(windows))]
    {
        let _ = cal;
        Err("Auto-Importer disponível apenas no Windows.".to_string())
    }
}

/// Moves the mouse over the detected Search Box and 1st Card (without clicking) so the user can verify aim.
pub fn test_aim_points(cal: &ImporterCalibration) -> Result<GameWindowStatus, String> {
    #[cfg(windows)]
    {
        let status = win_input::focus_master_duel(cal)?;
        thread::sleep(Duration::from_millis(350));
        let (tx, ty) = win_input::unowned_toggle_coords(&status);
        win_input::move_cursor_only(tx, ty);
        thread::sleep(Duration::from_millis(650));
        win_input::move_cursor_only(status.search_x, status.search_y);
        thread::sleep(Duration::from_millis(650));
        win_input::move_cursor_only(status.first_card_x, status.first_card_y);
        thread::sleep(Duration::from_millis(650));
        Ok(status)
    }
    #[cfg(not(windows))]
    {
        let _ = cal;
        Err("Auto-Importer disponível apenas no Windows.".to_string())
    }
}

pub fn run_deck_auto_import(
    cards: Vec<PreparedImportCard>,
    cal: ImporterCalibration,
    cancel_flag: Arc<AtomicBool>,
) -> Result<AutoImportSummary, String> {
    cancel_flag.store(false, Ordering::SeqCst);

    #[cfg(windows)]
    let _initial_status = win_input::focus_master_duel(&cal)?;

    // Short 900ms settle delay after focusing Master Duel (press ESC anytime to abort)
    if !sleep_check_cancel(900, &cancel_flag) {
        return Ok(AutoImportSummary {
            completed: false,
            cancelled: true,
            cards_processed: 0,
            total_cards: cards.len(),
            message: "Auto-Import cancelado antes de iniciar.".to_string(),
        });
    }

    // Before exporting cards: check if "Show Unowned Cards" button is green (ON) or black (OFF).
    // If black (OFF), automatically click it to turn it green!
    let mut toggled_unowned = false;
    #[cfg(windows)]
    {
        let live_now = win_input::query_window_status(&cal);
        if live_now.found && !live_now.minimized {
            if let Ok(clicked) = win_input::ensure_unowned_cards_toggle_active(&live_now) {
                toggled_unowned = clicked;
                if clicked && !sleep_check_cancel(320, &cancel_flag) {
                    return cancelled_summary(0, cards.len());
                }
            }
        }
    }

    let step_delay = cal.step_delay_ms.clamp(180, 2000);
    let click_delay = cal.click_delay_ms.clamp(120, 1000);
    let total = cards.len();
    let mut processed = 0usize;

    for card in &cards {
        if should_abort(&cancel_flag) {
            return cancelled_summary(processed, total);
        }

        #[cfg(windows)]
        {
            // Re-query Master Duel window coordinates & foreground focus before EVERY card!
            if !win_input::is_master_duel_foreground() {
                return Ok(AutoImportSummary {
                    completed: false,
                    cancelled: true,
                    cards_processed: processed,
                    total_cards: total,
                    message: format!(
                        "Auto-Import pausado por segurança ({}/{} cartas): o Master Duel perdeu o foco da janela.",
                        processed, total
                    ),
                });
            }
            let live = win_input::query_window_status(&cal);
            if !live.found || live.minimized {
                return Err(format!(
                    "Janela do Master Duel foi fechada ou minimizada após {}/{} cartas.",
                    processed, total
                ));
            }

            // 1. Focus Master Duel search box (single clean click)
            win_input::move_and_left_click(live.search_x, live.search_y);
            if !sleep_check_cancel(75, &cancel_flag) {
                return cancelled_summary(processed, total);
            }

            // 2. Multi-frame Ctrl+A + Backspace, then type smart disambiguated query + Enter
            win_input::select_all_and_delete();
            if !sleep_check_cancel(35, &cancel_flag) {
                return cancelled_summary(processed, total);
            }
            win_input::type_unicode_text(&card.search_query);
            if !sleep_check_cancel(40, &cancel_flag) {
                return cancelled_summary(processed, total);
            }
            win_input::press_enter();

            // 3. Wait base `step_delay` (default 700ms) for Master Duel search results to filter,
            // AND if the card is still loading (empty navy slot), wait adaptively up to +1100ms more!
            if !sleep_check_cancel(step_delay, &cancel_flag) {
                return cancelled_summary(processed, total);
            }
            let mut extra_waited = 0u64;
            let mut card_visible = win_input::is_first_card_slot_populated(
                live.first_card_x,
                live.first_card_y,
            );
            while !card_visible && extra_waited < 1100 {
                if !sleep_check_cancel(85, &cancel_flag) {
                    return cancelled_summary(processed, total);
                }
                extra_waited += 85;
                card_visible = win_input::is_first_card_slot_populated(
                    live.first_card_x,
                    live.first_card_y,
                );
            }
            if extra_waited > 0 && card_visible && !sleep_check_cancel(80, &cancel_flag) {
                return cancelled_summary(processed, total);
            }

            // 4. Right-click the 1st card with fresh pointer hover + 58ms hold + cooldown pause!
            if card_visible {
                let copies = card.amount.min(3);
                for copy_idx in 0..copies {
                    if should_abort(&cancel_flag) {
                        return cancelled_summary(processed, total);
                    }
                    win_input::right_click_card(live.first_card_x, live.first_card_y, copy_idx == 0);
                    if copy_idx + 1 < copies && !sleep_check_cancel(click_delay, &cancel_flag) {
                        return cancelled_summary(processed, total);
                    }
                }
            }
        }

        processed += 1;
        if !sleep_check_cancel(120, &cancel_flag) {
            return cancelled_summary(processed, total);
        }
    }

    let disambig_count = cards.iter().filter(|c| c.disambiguated).count();
    let mut notes: Vec<String> = Vec::new();
    if disambig_count > 0 {
        notes.push(format!("{} buscas desambiguadas", disambig_count));
    }
    if toggled_unowned {
        notes.push("cartas não possuídas ativado".to_string());
    }
    let extra_info = if notes.is_empty() {
        String::new()
    } else {
        format!(" ({})", notes.join(" • "))
    };

    Ok(AutoImportSummary {
        completed: true,
        cancelled: false,
        cards_processed: processed,
        total_cards: total,
        message: format!(
            "Concluído! {} cartas únicas enviadas para o Master Duel{}.",
            processed, extra_info
        ),
    })
}

fn cancelled_summary(processed: usize, total: usize) -> Result<AutoImportSummary, String> {
    Ok(AutoImportSummary {
        completed: false,
        cancelled: true,
        cards_processed: processed,
        total_cards: total,
        message: format!(
            "Auto-Import interrompido (ESC) após {}/{} cartas únicas.",
            processed, total
        ),
    })
}

fn should_abort(cancel_flag: &Arc<AtomicBool>) -> bool {
    if cancel_flag.load(Ordering::SeqCst) {
        return true;
    }
    #[cfg(windows)]
    {
        if win_input::is_escape_pressed() {
            cancel_flag.store(true, Ordering::SeqCst);
            return true;
        }
    }
    false
}

fn sleep_check_cancel(total_ms: u64, cancel_flag: &Arc<AtomicBool>) -> bool {
    let mut elapsed = 0u64;
    while elapsed < total_ms {
        if should_abort(cancel_flag) {
            return false;
        }
        let step = (total_ms - elapsed).min(50);
        thread::sleep(Duration::from_millis(step));
        elapsed += step;
    }
    !should_abort(cancel_flag)
}
