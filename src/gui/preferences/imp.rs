use adw::{subclass::prelude::*, ActionRow};
use glib::{prelude::ObjectExt, Properties};
use gtk::{Button, CompositeTemplate, SpinButton, Switch};

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
    #[template_child]
    pub src_row: TemplateChild<ActionRow>,
    #[template_child]
    pub bkp_row: TemplateChild<ActionRow>,
    #[template_child]
    pub src_button: TemplateChild<Button>,
    #[template_child]
    pub bkp_button: TemplateChild<Button>,
}

#[glib::object_subclass]
impl ObjectSubclass for DorstPreferences {
    const NAME: &'static str = "DorstPreferences";
    type Type = super::DorstPreferences;
    type ParentType = adw::PreferencesDialog;

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
impl AdwDialogImpl for DorstPreferences {}
impl PreferencesDialogImpl for DorstPreferences {}
