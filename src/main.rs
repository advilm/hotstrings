use std::ffi::CStr;
use x11::xlib::XKeysymToString;
use xcb::{
    xinput::{self, Device},
    Connection,
    Extension::Input,
};

mod read_map;

fn main() -> xcb::Result<()> {
    let hotstring_map = read_map::read_map("map").unwrap();

    let (conn, screen_num) = Connection::connect_with_extensions(None, &[Input], &[])?;
    let setup = conn.get_setup();
    let root = setup.roots().nth(screen_num as usize).unwrap().root();

    conn.wait_for_reply(conn.send_request(&xinput::XiQueryVersion {
        major_version: 2,
        minor_version: 0,
    }))
    .expect("XI2 not supported");

    let (keysyms, keysyms_per_keycode) = get_keysym_info(&conn);
    let min_keycode: u32 = setup.min_keycode().into();

    let cookie = conn.send_request(&xinput::ListInputDevices {});
    let device_list = conn.wait_for_reply(cookie)?;

    let device = {
        let mut device: Option<Device> = None;
        for (i, dev) in device_list.devices().iter().enumerate() {
            let name = device_list.names().nth(i).unwrap().name().to_utf8();
            if name.contains("Set 2 keyboard") {
                device = Some(xinput::Device::from_id(dev.device_id() as _));
                break;
            }
        }
        device.expect("could not find a keyboard")
    };

    let cookie = conn.send_request(&xinput::OpenDevice {
        device_id: device.id() as u8,
    });
    conn.wait_for_reply(cookie)?;

    conn.send_request(&xinput::XiSelectEvents {
        window: root,
        masks: &[xinput::EventMaskBuf::new(
            device,
            &[xinput::XiEventMask::KEY_PRESS | xinput::XiEventMask::KEY_RELEASE],
        )],
    });

    conn.flush()?;

    let mut state = String::new();
    loop {
        match conn.wait_for_event()? {
            xcb::Event::Input(xinput::Event::KeyPress(key_press)) => {
                let keycode = key_press.detail();
                let index = (keycode - min_keycode) * keysyms_per_keycode as u32;
                let keysym = keysyms[index as usize];

                // range of printable ascii characters
                if let 32..=126 = keysym {
                    // limit string size to 30
                    if state.len() == 30 {
                        state.remove(0);
                    }
                    state.push(keysym as u8 as char);

                    let val = hotstring_map.iter().find(|vec| state.ends_with(&vec[0]));
                    if let Some(val) = val {
                        for _ in val[0].char_indices() {
                            fake_input_keycode(&conn, 22, root, device.id(), 3);
                            fake_input_keycode(&conn, 22, root, device.id(), 2);
                            fake_input_keycode(&conn, 22, root, device.id(), 3);
                        }
                        for character in val[1].as_bytes() {
                            let index = keysyms
                                .iter()
                                .position(|&keysym| keysym == *character as u32)
                                .expect("failed to find keysym");
                            let keycode = index as u32 / keysyms_per_keycode as u32 + min_keycode;

                            fake_input_keycode(&conn, keycode, root, device.id(), 3);
                            fake_input_keycode(&conn, keycode, root, device.id(), 2);
                            fake_input_keycode(&conn, keycode, root, device.id(), 3);
                        }
                        conn.flush()?;
                    }
                }

                println!(
                    "Keycode: {}\nKeysym: {}\nKeysymString: {}\n",
                    keycode,
                    keysym,
                    keysym_to_string(keysym)
                );
            }
            _ => {}
        }
    }
}

pub fn get_keysym_info(conn: &xcb::Connection) -> (Vec<u32>, u8) {
    let setup = conn.get_setup();
    let cookie = conn.send_request(&xcb::x::GetKeyboardMapping {
        first_keycode: setup.min_keycode(),
        count: setup.max_keycode() - setup.min_keycode() + 1,
    });

    let reply = conn.wait_for_reply(cookie).unwrap();

    (reply.keysyms().into(), reply.keysyms_per_keycode())
}

pub fn keysym_to_string(keysym: u32) -> String {
    unsafe {
        let cstr = CStr::from_ptr(XKeysymToString(keysym as u64));
        cstr.to_str().unwrap().into()
    }
}

pub fn fake_input_keycode(
    conn: &xcb::Connection,
    keycode: u32,
    root: xcb::x::Window,
    deviceid: u16,
    input_type: u8,
) {
    conn.send_request(&xcb::xtest::FakeInput {
        r#type: input_type,
        detail: keycode as u8,
        time: 0,
        root: root,
        root_x: 0,
        root_y: 0,
        deviceid: deviceid as u8,
    });
}
