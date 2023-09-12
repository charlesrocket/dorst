use gtk::{glib, subclass::prelude::*, CompositeTemplate, Image, Label, ProgressBar, Revealer};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/hellbyte/dorst/repo_box.ui")]
pub struct RepoBox {
    #[template_child]
    pub status_image: TemplateChild<Image>,
    #[template_child]
    pub name: TemplateChild<Label>,
    #[template_child]
    pub link: TemplateChild<Label>,
    #[template_child]
    pub branch: TemplateChild<Label>,
    #[template_child]
    pub pb_revealer: TemplateChild<Revealer>,
    #[template_child]
    pub branch_revealer: TemplateChild<Revealer>,
    #[template_child]
    pub status_revealer: TemplateChild<Revealer>,
    #[template_child]
    pub progress_bar: TemplateChild<ProgressBar>,
}

#[glib::object_subclass]
impl ObjectSubclass for RepoBox {
    const NAME: &'static str = "DorstRepoBox";
    type Type = super::RepoBox;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for RepoBox {}

impl WidgetImpl for RepoBox {}

impl BoxImpl for RepoBox {}
