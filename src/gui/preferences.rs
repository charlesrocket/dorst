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

    pub fn setup_settings(&self, window: &crate::gui::window::Window) {
        let logs_switch = self.imp().logs_switch.get();

        #[cfg(feature = "logs")]
        {
            window
                .bind_property("logs", &logs_switch, "state")
                .bidirectional()
                .sync_create()
                .build();

            logs_switch.set_active(window.logs());
        }

        #[cfg(not(feature = "logs"))]
        logs_switch.set_sensitive(false);

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

    #[cfg(feature = "logs")]
    #[gtk::test]
    fn logging() {
        let window = window();
        let pref_window = preferences_window();

        pref_window.setup_settings(&window);

        window.set_logs(true);
        assert!(pref_window.imp().logs_switch.state());

        window.set_logs(false);
        assert!(!pref_window.imp().logs_switch.state());
    }

    #[gtk::test]
    fn task_limiter() {
        let window = window();
        let pref_window = preferences_window();

        pref_window.setup_settings(&window);

        window.set_task_limiter(true);
        assert!(pref_window.imp().limiter_switch.state());

        window.set_task_limiter(false);
        assert!(!pref_window.imp().limiter_switch.state());
    }

    #[gtk::test]
    fn pool_limit() {
        let window = window();
        let pref_window = preferences_window();

        pref_window.setup_settings(&window);
        window.set_thread_pool(3);

        assert!(window.thread_pool() == 3);
        assert!(pref_window.pool_limit() == 3);
    }
}
