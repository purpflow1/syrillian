use crate::{AudioReceiver, Collider3D, RigidBodyComponent, ThirdPersonCharacterController};
use syrillian::World;
use syrillian::core::GameObjectId;
use syrillian::engine::prefabs::Prefab;
use syrillian::physics::rapier3d::geometry::SharedShape;
use syrillian::tracing::warn;

pub struct ThirdPersonPlayerPrefab;

impl Prefab for ThirdPersonPlayerPrefab {
    fn prefab_name(&self) -> &'static str {
        "Third Person Player"
    }

    fn build(&self, world: &mut World) -> GameObjectId {
        // Prepare camera
        let camera = world.new_camera();
        let mut camera_obj = camera.parent();
        camera_obj.transform.set_position(0.0, 1.5, 4.0);
        camera_obj.add_component::<AudioReceiver>();

        // Prepare character controller
        let mut player = world.new_object(self.prefab_name());
        player.transform.set_position(0.0, 1.25, 0.0);

        player
            .add_component::<Collider3D>()
            .collider_mut()
            .unwrap()
            .set_shape(SharedShape::capsule_y(0.9, 0.3));

        if let Some(rigid_body) = player.add_component::<RigidBodyComponent>().body_mut() {
            rigid_body.enable_ccd(true);
            rigid_body.set_linvel([0.0, 0.0, 0.0].into(), true);
            rigid_body.set_angvel([0.0, 0.0, 0.0].into(), true);
        } else {
            warn!("Not able to set rigid body properties for Third Person Player Prefab");
        }

        player.add_child(camera_obj);
        player.add_component::<ThirdPersonCharacterController>();

        world.set_active_camera(camera);

        player
    }
}
