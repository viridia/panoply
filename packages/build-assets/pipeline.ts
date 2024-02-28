/// <reference types="overrun/overrun" />
import { directory, target, DirectoryTask, source } from "overrun";
import { NodeIO, Document, Logger } from "@gltf-transform/core";
import { resample, dedup, draco } from "@gltf-transform/functions";
import path from "path";

const srcRoot = path.resolve(__dirname, "../../artwork");
const dstRoot = path.resolve(__dirname, "../../assets");
const io = new NodeIO();
io.setLogger(new Logger(Logger.Verbosity.ERROR));

// .glb model optimization pass.
const modelOpt = async (src: Buffer): Promise<Buffer> => {
  const gltf = await io.readBinary(src);
  await gltf.transform(resample(), dedup(), draco());
  return Buffer.from(await io.writeBinary(gltf));
};

// // Character model files.
// target(
//   'characters',
//   directory(srcRoot, 'characters')
//     .match('*.glb')
//     .map(src => src.transform(modelOpt).dest({ root: dstRoot }))
// );

// target(
//   'props',
//   directory(srcRoot, 'props')
//     .match('*.glb')
//     .map(src => src.transform(modelOpt).dest({ root: dstRoot }))
// );

// // Scenery files.
// target(
//   'scenery',
//   directory(srcRoot, 'scenery')
//     .match('*.glb')
//     .map(src => src.transform(modelOpt).dest({ root: dstRoot }))
// );

// // Sound effects files.
// target(
//   'audio',
//   directory({ root: `${srcRoot}/sfx` })
//     .match('*.ogg')
//     .map(src => src.dest({ root: `${dstRoot}/audio/fx` }))
// );

// Flora function, merges .glb files.
function flora(targetName: string, relativeSrcDir: string, outFile: string) {
  target(
    targetName,
    directory(srcRoot, relativeSrcDir)
      .match("*.glb")
      .reduce(new Document(), async (doc, next) => {
        const gltf = await io.readBinary(await next.read());
        // throw new Error(JSON.stringify(gltf.getRoot()));
        return doc.merge(gltf);
      })
      .transform(async (doc) => {
        const buffer = doc.getRoot().listBuffers()[0];
        doc
          .getRoot()
          .listAccessors()
          .forEach((a) => a.setBuffer(buffer));
        doc
          .getRoot()
          .listBuffers()
          .forEach((b, index) => (index > 0 ? b.dispose() : null));
        doc
          .getRoot()
          .listMaterials()
          .forEach((m) => {
            if (m.getAlphaMode() === "BLEND") {
              m.setAlphaMode("MASK");
              m.setAlphaCutoff(0.1);
            }
          });

        const cameras = doc.getRoot().listCameras();
        cameras.forEach((camera) => camera.detach());

        return Buffer.from(await io.writeBinary(doc));
      })
      .dest(dstRoot, outFile)
  );
}

function trees(group: string) {
  flora(
    `trees-${group}`,
    `flora/trees-${group}/`,
    `terrain/models/${group}.glb`
  );
}

trees("temperate");
trees("coniferous");
trees("dead");
trees("arctic");
trees("desert");
flora("flora-shrubs", "flora/shrubs", "terrain/models/shrubs.glb");
flora("flora-crops", "flora/crops", "terrain/models/crops.glb");
flora("flora-bushes", "flora/bushes", "terrain/models/bushes.glb");

// // Textures
// target(
//   'textures',
//   directory(srcRoot, 'textures')
//     .match('*.png')
//     .map(src => src.dest({ root: dstRoot }))
// );

// target(
//   'textures-envmap',
//   source(srcRoot, 'textures/envmap/skybox.png').dest(dstRoot, 'textures/skybox.png')
// );

// target(
//   'textures-envmap',
//   source(srcRoot, 'textures/envmap/checkers.png').dest(dstRoot, 'textures/checkers.png')
// );

// // Textures index file.
// target(
//   'textures-index',
//   directory(srcRoot, 'textures').pipe(catalogIndex('*.png')).dest(dstRoot, 'textures/index.json')
// );

// // Maps
// target(
//   'textures-map',
//   directory(`${srcRoot}/images`, 'maps')
//     .match('*.png')
//     .map(src => src.dest({ root: dstRoot }))
// );

// // Fonts
// target(
//   'fonts',
//   directory(srcRoot, 'fonts')
//     .match('*.ttf')
//     .map(src => src.dest({ root: dstRoot }))
// );
