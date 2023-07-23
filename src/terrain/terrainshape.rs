use super::PLOT_LENGTH;
use serde::{Deserialize, Serialize};

const PLOT_STRIDE: usize = PLOT_LENGTH as usize;
const PLOT_AREA: usize = PLOT_STRIDE * PLOT_STRIDE;
type HeightArray = [i16; PLOT_AREA];
type FloraArray = [u8; PLOT_AREA];

#[derive(Serialize, Deserialize, Debug)]
pub struct TerrainShape {
    #[serde(with = "serde_arrays", alias = "terrainHeight")]
    pub height: HeightArray,
    #[serde(with = "serde_arrays")]
    pub flora: FloraArray,

    // public readonly lakes: Box2[] = [];
    // public needsUpdateVertices = false;
    #[serde(alias = "hasTerrain")]
    pub has_terrain: bool,
    #[serde(alias = "hasWater")]
    pub has_water: bool,
}

impl Drop for TerrainShape {
    fn drop(&mut self) {
        println!("TerrainShape dropped.");
    }
}

impl TerrainShape {
    pub fn new(&mut self) {
        self.height.fill(0);
        self.flora.fill(0);
        self.has_terrain = false;
        self.has_water = false;
    }

    /// Get terrain height (non-interpolated).
    pub fn height_at(&self, x: usize, y: usize) -> i16 {
        assert!(x < PLOT_STRIDE);
        assert!(y < PLOT_STRIDE);
        return self.height[x + y * PLOT_STRIDE];
    }

    pub fn flora_at(&self, x: usize, y: usize) -> u8 {
        assert!(x < PLOT_STRIDE);
        assert!(y < PLOT_STRIDE);
        return self.flora[x + y * PLOT_STRIDE];
    }

    //   public fillHeight(area: Box2, height: number): TerrainPlot {
    //     const { min, max } = clampBox(area);
    //     height = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, height));
    //     for (let y = min.y; y <= max.y; y += 1) {
    //       const rowOffset = y * PLOT_VERTEX_STRIDE;
    //       this.terrainHeight.fill(height, rowOffset + min.x, rowOffset + max.x + 1);
    //     }
    //     return this;
    //   }

    //   public fillFlora(area: Box2, flora: FloraType): this {
    //     const { min, max } = clampBox(area);
    //     for (let y = min.y; y < max.y; y += 1) {
    //       const rowOffset = y * PLOT_LENGTH;
    //       this.flora.fill(flora, rowOffset + min.x, rowOffset + max.x);
    //     }
    //     return this;
    //   }
}
