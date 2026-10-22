/// <reference types="overrun/overrun" />
import { directory, target, DirectoryTask, source } from "overrun";
import { NodeIO, Document, Logger } from "@gltf-transform/core";
import { resample, dedup, unpartition } from "@gltf-transform/functions";
import { KHRMaterialsUnlit, Unlit } from "@gltf-transform/extensions";
import path from "path";
import zlib from "zlib";
import { PNG } from "pngjs";
import {
  createDefaultContainer,
  write as writeKtx2,
  KHR_DF_CHANNEL_RGBSDA_ALPHA,
  KHR_DF_CHANNEL_RGBSDA_BLUE,
  KHR_DF_CHANNEL_RGBSDA_GREEN,
  KHR_DF_CHANNEL_RGBSDA_RED,
  KHR_DF_FLAG_ALPHA_STRAIGHT,
  KHR_DF_MODEL_RGBSDA,
  KHR_DF_PRIMARIES_BT709,
  KHR_DF_SAMPLE_DATATYPE_LINEAR,
  KHR_DF_TRANSFER_SRGB,
  KHR_SUPERCOMPRESSION_ZLIB,
  VK_FORMAT_R8G8B8A8_SRGB,
} from "ktx-parse";

const srcRoot = path.resolve(__dirname, "../../artwork");
const dstRoot = path.resolve(__dirname, "../../assets");
const io = new NodeIO();
io.setLogger(new Logger(Logger.Verbosity.ERROR));

// .glb model optimization pass.
const modelOpt = async (src: Buffer): Promise<Buffer> => {
  const gltf = await io.readBinary(src);
  await gltf.transform(
    (doc) => {
      // Split the default scene into separate named scenes
      let unlit: Unlit | undefined = undefined;
      let scene = doc.getRoot().getDefaultScene();
      if (scene) {
        doc.getRoot().setDefaultScene(null);
        let armature = scene
          .listChildren()
          .find((c) => c.getName().includes("Armature"));
        for (let child of scene.listChildren()) {
          if (child === armature) {
            continue;
          }
          // console.log(child.getName());
          let skin = child.getSkin();
          let namedScene = doc.createScene(child.getName()).addChild(child);
          if (armature && skin) {
            namedScene.addChild(armature);
          }
        }
      }
      for (let mat of doc.getRoot().listMaterials()) {
        if (mat.getExtras().black || mat.getExtras().unlit) {
          if (!unlit) {
            let ext = doc.createExtension(KHRMaterialsUnlit);
            unlit = ext.createUnlit();
          }
          mat.setExtension("KHR_materials_unlit", unlit);
        }
      }
      return doc;
    },
    resample(),
    dedup()
  );
  return Buffer.from(await io.writeBinary(gltf));
};

// Character model files.
target(
  "characters",
  directory(srcRoot, "characters")
    .match("*.glb")
    .map((src) => src.transform(modelOpt).dest({ root: dstRoot }))
);

// target(
//   'props',
//   directory(srcRoot, 'props')
//     .match('*.glb')
//     .map(src => src.transform(modelOpt).dest({ root: dstRoot }))
// );

// Scenery files.
target(
  "scenery",
  directory({ root: `${srcRoot}/scenery` })
    .match("*.glb")
    .map((src) =>
      src.transform(modelOpt).dest({ root: `${dstRoot}/scenery/models` })
    )
);

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
            } else {
              m.setExtras({ outline: 0.015 });
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

// arbor_day flora: merges .gltf files (each with its .bin and leaf .png) into one .glb, with
// the leaf textures embedded. arbor_day writes materials as the game reads them, so unlike
// flora() nothing is patched. flora-new and models-new mirror flora and models, so a biome
// entry switches between the two by its path.
function arborDayTrees(group: string) {
  target(
    `trees-new-${group}`,
    directory(srcRoot, `flora-new/trees-${group}`)
      .match("*.gltf")
      .reduce(new Document(), async (doc, next) =>
        // Read by path, not buffer, so the .bin and .png the .gltf references resolve.
        doc.merge(await io.read(next.path.full))
      )
      .transform(async (doc) => {
        await doc.transform(unpartition(), dedup());
        // Leaf textures embedded as KTX2 with mipmaps; Bevy decodes them by MIME type (X27).
        for (const texture of doc.getRoot().listTextures()) {
          const image = texture.getImage();
          if (image && texture.getMimeType() === "image/png") {
            texture
              .setImage(new Uint8Array(await pngToKtx2(Buffer.from(image))))
              .setMimeType("image/ktx2");
          }
        }
        return Buffer.from(await io.writeBinary(doc));
      })
      .dest(dstRoot, `terrain/models-new/${group}.glb`)
  );
}

arborDayTrees("temperate");

// Texture conversion: PNG to KTX2 with mipmaps.
const SRGB_TO_LINEAR = new Float32Array(256);
for (let i = 0; i < 256; i++) {
  const c = i / 255;
  SRGB_TO_LINEAR[i] =
    c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function linearToSrgb8(c: number): number {
  const s = c <= 0.0031308 ? c * 12.92 : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
  return Math.max(0, Math.min(255, Math.round(s * 255)));
}

interface MipLevel {
  width: number;
  height: number;
  /** RGB in linear light, alpha as stored; four floats per texel. */
  texels: Float32Array;
}

/** Halve a level with a 2x2 box filter. Wraps at the edges, since the textures tile. */
function downsample(level: MipLevel): MipLevel {
  const width = Math.max(1, level.width >> 1);
  const height = Math.max(1, level.height >> 1);
  const texels = new Float32Array(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      for (let c = 0; c < 4; c++) {
        let sum = 0;
        for (let dy = 0; dy < 2; dy++) {
          for (let dx = 0; dx < 2; dx++) {
            const sx = (x * 2 + dx) % level.width;
            const sy = (y * 2 + dy) % level.height;
            sum += level.texels[(sy * level.width + sx) * 4 + c];
          }
        }
        texels[(y * width + x) * 4 + c] = sum / 4;
      }
    }
  }
  return { width, height, texels };
}

function encodeMipLevel(level: MipLevel): Uint8Array {
  const bytes = new Uint8Array(level.width * level.height * 4);
  for (let i = 0; i < bytes.length; i += 4) {
    bytes[i] = linearToSrgb8(level.texels[i]);
    bytes[i + 1] = linearToSrgb8(level.texels[i + 1]);
    bytes[i + 2] = linearToSrgb8(level.texels[i + 2]);
    bytes[i + 3] = Math.round(level.texels[i + 3] * 255);
  }
  return bytes;
}

/** Convert a PNG to a KTX2 texture, RGBA8 sRGB with a full mip chain and zlib supercompression.
    Mips are averaged in linear light, so they do not darken. Every texture is stored as sRGB,
    since Bevy loads PNGs as sRGB by default and the shaders were tuned against that. */
const pngToKtx2 = async (src: Buffer): Promise<Buffer> => {
  const png = PNG.sync.read(src);
  const { width, height } = png;
  const base = new Float32Array(width * height * 4);
  for (let i = 0; i < base.length; i += 4) {
    base[i] = SRGB_TO_LINEAR[png.data[i]];
    base[i + 1] = SRGB_TO_LINEAR[png.data[i + 1]];
    base[i + 2] = SRGB_TO_LINEAR[png.data[i + 2]];
    base[i + 3] = png.data[i + 3] / 255;
  }

  const levels: MipLevel[] = [{ width, height, texels: base }];
  while (
    levels[levels.length - 1].width > 1 ||
    levels[levels.length - 1].height > 1
  ) {
    levels.push(downsample(levels[levels.length - 1]));
  }

  const container = createDefaultContainer();
  container.vkFormat = VK_FORMAT_R8G8B8A8_SRGB;
  container.typeSize = 1;
  container.pixelWidth = width;
  container.pixelHeight = height;
  container.levelCount = levels.length;
  container.supercompressionScheme = KHR_SUPERCOMPRESSION_ZLIB;
  container.levels = levels.map((level) => {
    const bytes = encodeMipLevel(level);
    const levelData = new Uint8Array(zlib.deflateSync(bytes));
    return { levelData, uncompressedByteLength: bytes.length };
  });
  const channels = [
    KHR_DF_CHANNEL_RGBSDA_RED,
    KHR_DF_CHANNEL_RGBSDA_GREEN,
    KHR_DF_CHANNEL_RGBSDA_BLUE,
    // Alpha is not sRGB-encoded.
    KHR_DF_CHANNEL_RGBSDA_ALPHA | KHR_DF_SAMPLE_DATATYPE_LINEAR,
  ];
  container.dataFormatDescriptor = [
    {
      ...container.dataFormatDescriptor[0],
      colorModel: KHR_DF_MODEL_RGBSDA,
      colorPrimaries: KHR_DF_PRIMARIES_BT709,
      transferFunction: KHR_DF_TRANSFER_SRGB,
      flags: KHR_DF_FLAG_ALPHA_STRAIGHT,
      texelBlockDimension: [0, 0, 0, 0],
      bytesPlane: [4, 0, 0, 0, 0, 0, 0, 0],
      samples: channels.map((channelType, index) => ({
        bitOffset: index * 8,
        bitLength: 7,
        channelType,
        samplePosition: [0, 0, 0, 0],
        sampleLower: 0,
        sampleUpper: 255,
      })),
    },
  ];
  return Buffer.from(writeKtx2(container));
};

// Terrain textures (exported from Krita as PNG): KTX2 with mipmaps.
target(
  "terrain-textures",
  directory(srcRoot, "terrain/textures")
    .match("*.png")
    .map((src) =>
      src
        .transform(pngToKtx2)
        .dest((p) => p.withRoot(dstRoot).withExtension(".ktx2"))
    )
);

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

// // Maps
// target(
//   'textures-map',
//   directory(`${srcRoot}/images`, 'maps')
//     .match('*.png')
//     .map(src => src.dest({ root: dstRoot }))
// );
