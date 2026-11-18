"""Check the baked channel order, scalar palette, and native-resolution surface detail."""
import json
from pathlib import Path
import sys
import unittest
import numpy as np

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from relief_materials import bc4, bc1, blocks, downsample, lattice


def scalar(block):
    a,b=int(block[0]),int(block[1])
    values=[a,b]+([((7-i)*a+i*b)//7 for i in range(1,7)] if a>b else
                  [((5-i)*a+i*b)//5 for i in range(1,5)]+[0,255])
    selectors=int.from_bytes(block[2:8],'little')
    return np.array([values[(selectors>>(3*i))&7] for i in range(16)])


class ReliefMaterials(unittest.TestCase):
    def test_endpoint_selectors_and_linear_mips(self):
        v=np.repeat(np.arange(16,dtype=np.uint8)[None,:]*17,2,axis=0)
        for block in bc4(v):
            self.assertLessEqual(np.max(abs(scalar(block)-v[0])),19)
            self.assertEqual(scalar(block)[0],0)
            self.assertEqual(scalar(block)[15],255)
        rgb=np.broadcast_to(np.array([255,0,0],np.uint8),(1,16,3))
        self.assertEqual(bc1(rgb)[0,:4].tolist(),[0,248,0,248])
        a=np.zeros((2,2,5),np.uint8);a[1,:,:]=255
        s=np.arange(256,dtype=np.float32)/255
        mip=downsample(a,np.where(s<=.04045,s/12.92,((s+.055)/1.055)**2.4))
        self.assertEqual(mip[0,0].tolist(),[188,188,188,128,128])
        self.assertEqual(blocks(mip).shape,(1,16,5))

    def test_wrapped_noise_does_not_change_on_tile_edges(self):
        xy=np.arange(128,dtype=np.float32)/128
        np.testing.assert_array_equal(lattice(xy,.375,887,4139),lattice(xy+1,1.375,887,4139))

    def test_all_three_4k_materials_have_unique_native_texel_relief(self):
        folder=ROOT/'crates/worldcraft/embedded'
        report=json.loads((folder/'manifest.json').read_text())['relief']
        self.assertEqual((report['side'],report['mips']),(4096,13))
        def layer_bytes(side,block):
            return sum(max(1,((side>>i)+3)//4)**2*block for i in range(side.bit_length()))
        packed=(folder/'procedural-height-bc4.bin').read_bytes()
        self.assertEqual(len(packed),3*layer_bytes(4096,8))
        signals=[]
        for layer in range(3):
            offset=layer*layer_bytes(4096,8)
            patches=[]
            for row in [45,322,846]:
                at=offset+(row*1024+58)*8
                patches.extend([scalar(packed[j:j+8]).reshape(4,4) for j in range(at,at+8*96,8)])
            fine=np.stack(patches).astype(float)
            # Alternating second differences survive at the native texel scale.
            self.assertGreater(float(abs(np.diff(fine,n=2,axis=2)).mean()),.8)
            signals.append(fine)
        for a,b in zip(signals,signals[1:]):
            self.assertGreater(float(np.abs(a-b).mean()),8.0)

if __name__=='__main__': unittest.main()
