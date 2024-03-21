#![cfg_attr(rustfmt, rustfmt_skip)]

//#![warn(missing_docs)]
//#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]
//#![cfg_attr(docsrs, feature(doc_auto_cfg))]
//#![cfg_attr(rustfmt, rustfmt_skip)]
//
//#[cfg(feature = "staging")]
//#[macro_use]
//mod protocol_macro;

//#[cfg(feature = "staging")]
//pub mod workspace {
//    //! This protocol allows clients to monitor user idle status.
//
//    #[allow(missing_docs)]
//    pub mod v1 {
//        wayland_protocol!(
//            "./resources/ext-workspace-v1.xml",
//            []
//        );
//    }
//}
pub mod workspace {
    pub mod v1 {
        pub mod client {
            use wayland_client;
            // import objects from the core protocol if needed
            use wayland_client::protocol::*;
            
            // This module hosts a low-level representation of the protocol objects
            // you will not need to interact with it yourself, but the code generated
            // by the generate_client_code! macro will use it
            pub mod __interfaces {
                // import the interfaces from the core protocol if needed
                use smithay_client_toolkit::reexports::client::protocol::__interfaces::*;
                wayland_scanner::generate_interfaces!("./resources/ext-workspace-unstable-v1.xml");
            }
            use self::__interfaces::*;
            
            // This macro generates the actual types that represent the wayland objects of
            // your custom protocol
            wayland_scanner::generate_client_code!("./resources/ext-workspace-unstable-v1.xml");
        }
    }
}
