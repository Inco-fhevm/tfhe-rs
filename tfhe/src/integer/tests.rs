macro_rules! create_parametrized_test{
    (
        $name:ident {
            $($(#[$cfg:meta])* $param:ident),*
            $(,)?
        }
    ) => {
        ::paste::paste! {
            $(
                #[test]
                $(#[$cfg])*
                fn [<test_ $name _ $param:lower>]() {
                    $name($param)
                }
            )*
        }
    };
     ($name:ident)=> {
        create_parametrized_test!($name
        {
            #[cfg(not(feature = "__coverage"))]
            PARAM_MESSAGE_1_CARRY_1_KS_PBS,
            #[cfg(not(feature = "__coverage"))]
            PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            #[cfg(not(feature = "__coverage"))]
            PARAM_MESSAGE_3_CARRY_3_KS_PBS,
            #[cfg(not(feature = "__coverage"))]
            PARAM_MESSAGE_4_CARRY_4_KS_PBS,
            #[cfg(feature = "__coverage")]
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS
        });
    };
}
