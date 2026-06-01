pub mod CanvasScaler;
pub mod ContentSizeFitter;
pub mod EventSystem;
pub mod HorizontalOrVerticalLayoutGroup;
pub mod Image;
pub mod LayoutElement;
pub mod LayoutGroup;
pub mod LayoutRebuilder;
pub mod Text;
pub mod VerticalLayoutGroup;

pub fn init() {
    get_assembly_image_or_return!(image, "UnityEngine.UI.dll");

    Text::init(image);
    CanvasScaler::init(image);
    EventSystem::init(image);
    LayoutElement::init(image);
    LayoutRebuilder::init(image);
    Image::init(image);
    LayoutGroup::init(image);
    HorizontalOrVerticalLayoutGroup::init(image);
    ContentSizeFitter::init(image);
}
