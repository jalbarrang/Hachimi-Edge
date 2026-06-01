use crate::il2cpp::types::Il2CppImage;

mod AnGlobalData;
mod AnKeyParameter;
mod AnMeshInfoParameterGroup;
mod AnMeshParameter;
mod AnMeshParameterGroup;
mod AnMotionParameter;
mod AnMotionParameterGroup;
mod AnObjectParameterBase;
pub mod AnRoot;
mod AnRootParameter;
mod AnText;
mod AnTextParameter;

pub fn init(image: *const Il2CppImage) {
    AnText::init(image);
    AnMeshInfoParameterGroup::init(image);
    AnMeshParameter::init(image);
    AnRoot::init(image);
    AnMeshParameterGroup::init(image);
    AnRootParameter::init(image);
    AnMotionParameterGroup::init(image);
    AnMotionParameter::init(image);
    AnTextParameter::init(image);
    AnKeyParameter::init(image);
    AnObjectParameterBase::init(image);
    AnGlobalData::init(image);
}
