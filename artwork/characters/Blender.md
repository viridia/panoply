# Character Rigging in Blender - Recipe

- Create armature (Shift A + Armature)
- Edit mode
- Select bone - object properties / viewport display / Show in front
- Move to pevis
- Extrude spine / shoulders (e)
- Alt + p / disconnect
- Name bones
- Select left-side bones / right click / auto-name left to right
- Create KneeIK / Shin IK
  - Remove parent
  - Uncheck "deform"
- Pose mode
  - select shin
  - go to bone contraint properties
  - Add bone constraint - inverse kinematics
    - Target: armature
    - Bone: ikShin
    - Chain length: 2
    - Pole target: ikKnee
    - Pole angle: -90
    - Lock axes: Bone / Inverse Kinematics / Lock IK
  - Go to edit mode: select foot + heel, ctrl-P / Keep offset
  - Pose mode:
    - Select foot, add bone constraint / copy location from calf
      - Head/tail offset: 1
  - Symmetrize
  - Select mesh, add Armature modifier.
    - Set name of Armature

Videos:

- Rigging tutorial in 2 minutes: https://www.youtube.com/watch?v=Erqgl_PQyrk
- Custom bone shapes: https://www.youtube.com/watch?v=Dw0wssU1DXA
- IK in 2 minutes https://www.youtube.com/watch?v=Pt3-mHBCoQk
- IK shortcuts https://www.youtube.com/watch?v=Cu5TozPfsD4&t=5s
- Rigging Playlist: https://www.youtube.com/watch?v=JbQX8C3lrHE&list=PLZpDYt0cyiusytIPAOTPRzsa4GK6LgY3_&index=2
