use adw::subclass::prelude::*;
use glib::{ObjectExt, Properties};
use gtk::{CompositeTemplate, SpinButton, Switch};

use std::sync::{Arc, Mutex};

#[derive(Default, CompositeTemplate, Properties)]
#[properties(wrapper_type = super::DorstPreferences)]
#[template(resource = "/org/hellbyte/dorst/preferences.ui")]
pub struct DorstPreferences {
    #[property(get, set)]
    pub pool_limit: Arc<Mutex<u64>>,
    #[template_child]
    pub logs_switch: TemplateChild<Switch>,
    #[template_child]
    pub limiter_switch: TemplateChild<Switch>,
    #[template_child]
    pub limiter_button: TemplateChild<SpinButton>,
}

#[glib::object_subclass]
impl ObjectSubclass for DorstPreferences {
    const NAME: &'static str = "DorstPreferences";
    type Type = super::DorstPreferences;
    type ParentType = adw::PreferencesWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for DorstPreferences {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

#[gtk::template_callbacks]
impl DorstPreferences {
    #[template_callback]
    fn pool_limit(&self, button: &gtk::SpinButton) {
        self.pool_limit.set(button.value() as u64);
    }
}

impl WidgetImpl for DorstPreferences {}
impl WindowImpl for DorstPreferences {}
impl AdwWindowImpl for DorstPreferences {}
impl PreferencesWindowImpl for DorstPreferences {}
