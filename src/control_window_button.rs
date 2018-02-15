/*
 *  Me TV — It's TV for me computer.
 *
 *  A GTK+/GStreamer client for watching and recording DVB.
 *
 *  Copyright © 2017, 2018  Russel Winder
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use gio;
use gio::prelude::*;
use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

use about;
use channel_names;
use control_window::{CONTROL_WINDOW, ControlWindow};
use frontend_manager::{FrontendId, TuningId, Message};
use frontend_window::create_frontend_window;
use gstreamer_engine::GStreamerEngine;

use comboboxtext_extras::ComboBoxTextExtras;

/// A `ControlWindowButton` is a `gtk::Box` but there is no inheritance so use
/// a bit of composition.
pub struct ControlWindowButton {
    control_window: Rc<ControlWindow>,
    tuning_id: TuningId,
    pub widget: gtk::Box,
    frontend_button: gtk::ToggleButton,
    channel_selector: gtk::ComboBoxText,
    inhibitor: Cell<u32>,
    frontend_window: gtk::ApplicationWindow,
    engine: GStreamerEngine,
}

impl ControlWindowButton {
    /// Construct a new button representing an available front end.
    ///
    /// The adapter and frontend numbers for the label for a toggle button that is used
    /// to start and stop a frontend window displaying the stream for that frontend. Below
    /// is a drop down list button to select the channel to tune the front end to.
    ///
    /// This function is executed in the GTK event loop thread.
    pub fn new(control_window: &Rc<ControlWindow>, fei: FrontendId, channel_names: &Vec<String>, default_channel_name: &String) -> Rc<ControlWindowButton> {
        let tuning_id = TuningId{frontend: fei, channel: default_channel_name.clone()};
        let frontend_button = gtk::ToggleButton::new_with_label(format!("adaptor{}\nfrontend{}", tuning_id.frontend.adapter, tuning_id.frontend.frontend).as_ref());
        let channel_selector = gtk::ComboBoxText::new();
        for (_, name) in channel_names.iter().enumerate() {
            channel_selector.append_text(name);
        }
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.pack_start(&frontend_button, true, true, 0);
        widget.pack_start(&channel_selector, true, true, 0);
        let frontend_window = create_frontend_window(&control_window.window.get_application().unwrap());
        let f_w = frontend_window.clone();
        let engine = GStreamerEngine::new();
        let cwb = Rc::new(ControlWindowButton {
            control_window: control_window.clone(),
            tuning_id: tuning_id.clone(),
            widget,
            frontend_button: frontend_button.clone(),
            channel_selector,
            inhibitor: Cell::new(0),
            frontend_window,
            engine,
        });
        cwb.set_label(&tuning_id.channel);
        let close_action = gio::SimpleAction::new("close", None);
        close_action.connect_activate({
            let c_w_b =cwb.clone();
            move |_, _| c_w_b.toggle_button()
        });
        f_w.add_action(&close_action);
        frontend_button.connect_toggled({
            let c_w_b = cwb.clone();
            move |_| {
                c_w_b.toggle_button();
            }
        });
        cwb
    }

    /// Set the state of the control window button.
    fn set_label(&self, channel_name: &String) {
        self.channel_selector.set_active_text(self.tuning_id.channel.as_ref());
    }

    /// Toggle the button.
    fn toggle_button(&self) {
        println!("Frontend window button toggled.");
        CONTROL_WINDOW.with(|global|{
             if let Some(ref control_window) = *global.borrow() {
                 let app = control_window.window.get_application().unwrap();
                 if self.frontend_button.get_active() {
                     println!("Active");
                     if self.inhibitor.get() == 0 {
                         println!("Activating inactive window.");
                         self.inhibitor.set(app.inhibit(&self.frontend_window, gtk::ApplicationInhibitFlags::SUSPEND, "Me TV inhibits when playing a channel."));
                         self.frontend_window.show_all();
                     } else {
                         println!("Window being activated is already active.");
                     }
                 } else {
                     println!("Inactive");
                     if self.inhibitor.get() != 0 {
                         println!("Deactivating active window.");
                         app.uninhibit(self.inhibitor.get());
                         self.inhibitor.set(0);
                         self.frontend_window.hide();
                     } else {
                         println!("Window being deactivated is not active.");
                     }
                 }
             }
         });
    }
}
