use decoder::MAX_COMPONENTS;
use idct::dequantize_and_idct_block;
use parser::Component;
use std::mem;
use std::rc::Rc;

pub struct RowData {
    pub index: usize,
    pub component: Component,
    pub quantization_table: Rc<[u16; 64]>,
}

pub enum WorkerMsg<'a> {
    Start(RowData),
    AppendRow((usize, Vec<i16>)),
    GetResult((usize, &'a mut Vec<u8>)),
}

pub struct WorkerContext {
    offsets: [usize; MAX_COMPONENTS],
    results: Vec<Vec<u8>>,
    components: Vec<Option<Component>>,
    quantization_tables: Vec<Option<Rc<[u16; 64]>>>,
}

impl WorkerContext {
    pub fn new() -> WorkerContext {
        WorkerContext{
            offsets: [0; MAX_COMPONENTS],
            results: vec![Vec::new(); MAX_COMPONENTS],
            components: vec![None; MAX_COMPONENTS],
            quantization_tables: vec![None; MAX_COMPONENTS],
        }
    }
    
    pub fn process_message(&mut self, message: WorkerMsg) {
        
        match message {
            WorkerMsg::Start(data) => {
                assert!(self.results[data.index].is_empty());

                self.offsets[data.index] = 0;
                self.results[data.index].resize(data.component.block_size.width as usize * data.component.block_size.height as usize * 64, 0u8);
                self.components[data.index] = Some(data.component);
                self.quantization_tables[data.index] = Some(data.quantization_table);
            },
            WorkerMsg::AppendRow((index, data)) => {
                // Convert coefficients from a MCU row to samples.

                let component = self.components[index].as_ref().unwrap();
                let quantization_table = self.quantization_tables[index].as_ref().unwrap();
                let block_count = component.block_size.width as usize * component.vertical_sampling_factor as usize;
                let line_stride = component.block_size.width as usize * 8;

                assert_eq!(data.len(), block_count * 64);

                for i in 0..block_count {
                    let x = (i % component.block_size.width as usize) * 8;
                    let y = (i / component.block_size.width as usize) * 8;
                    dequantize_and_idct_block(&data[i * 64..(i + 1) * 64],
                                              quantization_table,
                                              line_stride,
                                              &mut self.results[index][self.offsets[index] + y * line_stride + x..]);
                }

                self.offsets[index] += data.len();
            },
            WorkerMsg::GetResult((index, result)) => {
                mem::swap(&mut self.results[index], result);
            },
        }
    }
}
