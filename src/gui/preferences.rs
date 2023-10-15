use gtk::{
    glib, prelude::*, subclass::prelude::*, Accessible, Buildable, ConstraintTarget, Native, Root,
    Widget, Window,
};

mod imp;

glib::wrapper! {
    pub struct DorstPreferences(ObjectSubclass<imp::DorstPreferences>)
        @extends Widget, Window, adw::Window,
        @implements Accessible, Buildable, ConstraintTarget, Native, Root;
}

impl DorstPreferences {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_settings(&self, window: &crate::gui::window::Window) {
        let limiter_switch = self.imp().limiter_switch.get();

        window
            .bind_property("task-limiter", &limiter_switch, "state")
            .bidirectional()
            .sync_create()
            .build();

        limiter_switch.set_active(window.task_limiter());

        let limiter_button = self.imp().limiter_button.get();

        window
            .bind_property("thread-pool", &limiter_button, "value")
            .bidirectional()
            .sync_create()
            .build();
    }
}

impl Default for DorstPreferences {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::window::tests::window;

    fn preferences_window() -> DorstPreferences {
        glib::Object::builder::<DorstPreferences>().build()
    }

    #[gtk::test]
    fn pool_limit() {
        let window = window();
        let pref_window = preferences_window();

        pref_window.set_settings(&window);
        window.set_thread_pool(3);

        assert!(window.thread_pool() == 3);
        assert!(pref_window.pool_limit() == 3);
    }
}
