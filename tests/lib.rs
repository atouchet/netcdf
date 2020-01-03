#![cfg(test)]

mod common;
use common::test_location;

#[test]
/// Use a path to open the netcdf file
fn use_path_to_open() {
    let path = test_location().join("simple_xy.nc");

    let _file = netcdf::open(path).unwrap();
}

#[test]
/// Use a string to open
fn use_string_to_open() {
    let f: String = test_location()
        .join("simple_xy.nc")
        .to_str()
        .unwrap()
        .to_string();
    let _file = netcdf::open(f).unwrap();
}

// Failure tests
#[test]
fn bad_filename() {
    let f = test_location().join("blah_stuff.nc");
    let res_file = netcdf::open(&f);
    assert!(
        if let netcdf::error::Error::Netcdf(2) = res_file.unwrap_err() {
            true
        } else {
            false
        }
    );
}

// Read tests
#[test]
fn root_dims() {
    let f = test_location().join("simple_xy.nc");

    let file = netcdf::open(&f).unwrap();
    assert_eq!(f.to_str().unwrap(), file.path().unwrap());

    assert_eq!(file.dimension("x").unwrap().len(), 6);
    assert_eq!(file.dimension("y").unwrap().len(), 12);
}

#[test]
fn access_through_deref() {
    let f = test_location().join("simple_xy.nc");

    let file = netcdf::open(&f).unwrap();

    assert_eq!(file.dimension("x").unwrap().len(), 6);
    assert_eq!(file.dimension("y").unwrap().len(), 12);

    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("derefmut.nc");
    let mut file = netcdf::create(&f).unwrap();

    file.add_dimension("time", 10).unwrap();

    assert_eq!(
        file.dimension("time")
            .expect("Could not find dimension")
            .len(),
        10
    );
}

#[test]
fn var_as_different_types() {
    let f = test_location().join("simple_xy.nc");

    let file = netcdf::open(&f).unwrap();

    let mut data = vec![0; 6 * 12];
    let var = &file
        .variable("data")
        .unwrap()
        .expect("Could not find variable");
    var.values_to(&mut data, None, None).unwrap();

    for (x, d) in data.iter().enumerate() {
        assert_eq!(*d, x as i32);
    }

    // do the same thing but cast to float
    let mut data = vec![0.0; 6 * 12];
    var.values_to(&mut data, None, None).unwrap();

    for (x, d) in data.iter().enumerate() {
        assert!((*d - x as f32).abs() < 1e-5);
    }
}

#[test]
fn test_index_fetch() {
    let f = test_location().join("simple_xy.nc");

    let file = netcdf::open(&f).unwrap();

    let var = &file
        .variable("data")
        .unwrap()
        .expect("Could not find variable");
    // Gets first value
    let first_val: i32 = var.value(None).unwrap();
    let other_val: i32 = var.value(Some(&[5, 3])).unwrap();

    assert_eq!(first_val, 0 as i32);
    assert_eq!(other_val, 63 as i32);
}

#[test]
#[cfg(feature = "ndarray")]
fn last_dim_varies_fastest() {
    let f = test_location().join("simple_xy.nc");

    let file = netcdf::open(&f).unwrap();

    let var = &file
        .variable("data")
        .unwrap()
        .expect("Could not find variable");
    let data = var.values::<i32>(None, None).unwrap();

    let nx = var.dimensions()[0].len();
    let ny = var.dimensions()[1].len();

    assert_eq!(nx, 6);
    assert_eq!(ny, 12);
    assert_eq!(nx * ny, data.len());

    for x in 0..nx {
        for y in 0..ny {
            let ind = x * ny + y;
            assert_eq!(data.as_slice().unwrap()[ind], ind as i32);
        }
    }
}

#[test]
fn variable_not_replacing() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("variable_not_replacing.nc");
    let mut f = netcdf::create(p).unwrap();

    f.add_variable::<u16>("a", &[]).unwrap();
    f.add_variable::<i16>("b", &[]).unwrap();
    f.add_variable::<u8>("a", &[]).unwrap_err();
    f.add_variable_from_identifiers::<i8>("b", &[]).unwrap_err();
}

#[test]
fn dimension_lengths() {
    let d = tempfile::tempdir().expect("Could not create tempdir");
    let path = d.path().join("dimension_lengths");
    let mut file = netcdf::create(path).expect("Could not create file");

    file.add_unlimited_dimension("unlim")
        .expect("Could not create dimension");
    file.add_dimension("lim", 10)
        .expect("Could not create dimension");

    let dim = &file.dimension("unlim").expect("Could not find dim");
    assert_eq!(dim.len(), 0);

    let dim = &file.dimension("lim").expect("Could not find dim");
    assert_eq!(dim.len(), 10);

    for dim in file.dimensions() {
        assert!(dim.len() == 0 || dim.len() == 10);
    }
}

#[test]
fn netcdf_error() {
    let path = ".";
    let err = netcdf::open(path).unwrap_err();

    use std::error::Error;
    println!("{} {:?}", err, err.source());

    let err: netcdf::error::Error = "hello".into();
    println!("{}", err);
    let err: netcdf::error::Error = String::from("hello").into();
    println!("{}", err);

    let d = tempfile::tempdir().expect("Could not get tempdir");
    let path = d.path().join("netcdf_error.nc");
    let mut file = netcdf::create(path).expect("Could not create file");

    file.add_variable::<i8>("var", &["v"]).unwrap_err();
    file.add_dimension("v", 3).expect("Could not add dimension");
    file.add_variable::<i8>("var", &["v"]).unwrap();
    file.add_variable::<i8>("var", &["v"]).unwrap_err();

    file.add_dimension("v", 2).unwrap_err();
    file.add_unlimited_dimension("v").unwrap_err();
}

#[test]
#[cfg(feature = "ndarray")]
fn ndarray_read_with_indices() {
    let f = test_location().join("pres_temp_4D.nc");
    let file = netcdf::open(f).unwrap();

    let var = &file.variable("pressure").unwrap().unwrap();

    let sizes = [
        var.dimensions()[0].len(),
        var.dimensions()[1].len(),
        1,
        var.dimensions()[2].len(),
    ];
    let indices = [0, 0, 3, 0];
    let values = var.values::<f32>(Some(&indices), Some(&sizes)).unwrap();

    assert_eq!(values.shape(), sizes);

    let indices = [0, 1, 3, 0];
    let sizes = [
        var.dimensions()[0].len(),
        var.dimensions()[1].len() - 1,
        2,
        var.dimensions()[2].len(),
    ];
    let values = var.values::<f32>(Some(&indices), Some(&sizes)).unwrap();
    assert_eq!(values.shape(), sizes);
}

#[test]
fn nc4_groups() {
    let f = test_location().join("simple_nc4.nc");

    let file = netcdf::open(&f).unwrap();

    let grp1 = &file.group("grp1").expect("Could not find group");
    assert_eq!(grp1.name().unwrap(), "grp1");

    let mut data = vec![0i32; 6 * 12];
    let var = &grp1.variable("data").unwrap();
    var.values_to(&mut data, None, None).unwrap();
    for (i, x) in data.iter().enumerate() {
        assert_eq!(*x, i as i32);
    }
}

#[test]
fn groups_put_extra() {
    let d = tempfile::tempdir().expect("Could not create tempdir");
    let p = d.path().join("groups_put_extra");
    let mut f = netcdf::create(p).unwrap();

    f.add_group("a").unwrap();
    f.add_group("b").unwrap();
    // Groups can not be added with the same name
    f.add_group("a").unwrap_err();
}

#[test]
fn create_group_dimensions() {
    let d = tempfile::tempdir().unwrap();
    let filepath = d.path().join("create_group.nc");
    let mut f = netcdf::create(filepath).unwrap();

    f.add_dimension("x", 20).unwrap();

    let g = &mut f.add_group("gp1").unwrap();

    g.add_dimension("x", 100).unwrap();
    g.add_variable::<u8>("y", &["x"]).unwrap();

    let gg = &mut g.add_group("gp2").unwrap();
    gg.add_variable::<i8>("y", &["x"]).unwrap();

    gg.add_dimension("x", 30).unwrap();
    gg.add_variable::<i8>("z", &["x"]).unwrap();

    assert_eq!(
        f.group("gp1")
            .expect("Could not find group")
            .variable("y")
            .unwrap()
            .dimensions()[0]
            .len(),
        100
    );
    assert_eq!(
        f.group("gp1")
            .expect("Could not find group")
            .group("gp2")
            .expect("Could not find group")
            .variable("y")
            .unwrap()
            .dimensions()[0]
            .len(),
        100
    );
    assert_eq!(
        f.group("gp1")
            .expect("Could not find group")
            .group("gp2")
            .expect("Could not find group")
            .variable("z")
            .expect("Could not find variable")
            .dimensions()[0]
            .len(),
        30
    );
}

// Write tests
#[test]
fn create() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("create.nc");

    let file = netcdf::create(&f).unwrap();
    assert_eq!(f.to_str().unwrap(), file.path().unwrap());
}

#[test]
#[cfg(feature = "ndarray")]
fn def_dims_vars_attrs() {
    use netcdf::attribute::AttrValue;
    let d = tempfile::tempdir().unwrap();
    {
        let f = d.path().join("def_dims_vars_attrs.nc");

        let mut file = netcdf::create(&f).unwrap();

        let dim1_name = "ljkdsjkldfs";
        let dim2_name = "dsfkdfskl";
        file.root_mut().add_dimension(dim1_name, 10).unwrap();
        file.root_mut().add_dimension(dim2_name, 20).unwrap();
        assert_eq!(
            file.root()
                .dimension(dim1_name)
                .expect("Could not find dimension")
                .len(),
            10
        );
        assert_eq!(
            file.root()
                .dimension(dim2_name)
                .expect("Could not find dimension")
                .len(),
            20
        );

        let var_name = "varstuff_int";
        let data: Vec<i32> = vec![42; 10 * 20];
        let var = &mut file
            .add_variable::<i32>(var_name, &[dim1_name, dim2_name])
            .unwrap();
        var.put_values(data.as_slice(), None, None).unwrap();
        assert_eq!(var.dimensions()[0].len(), 10);
        assert_eq!(var.dimensions()[1].len(), 20);

        let var_name = "varstuff_float";
        let data: Vec<f32> = vec![42.2; 10];
        let mut var = file.add_variable::<f32>(var_name, &[dim1_name]).unwrap();
        var.put_values(data.as_slice(), None, None).unwrap();
        assert_eq!(var.dimensions()[0].len(), 10);

        // test global attrs
        file.add_attribute("testattr1", 3).unwrap();
        file.add_attribute("testattr2", "Global string attr".to_string())
            .unwrap();

        // test var attrs
        let mut var = file.variable_mut(var_name).unwrap();
        var.add_attribute("varattr1", 5).unwrap();
        var.add_attribute("varattr2", "Variable string attr".to_string())
            .unwrap();
    }

    // now, read in the file we created and verify everything
    {
        use ndarray::ArrayD;
        let f = d.path().join("def_dims_vars_attrs.nc");

        let file = netcdf::open(&f).unwrap();

        // verify dimensions
        let dim1_name = "ljkdsjkldfs";
        let dim2_name = "dsfkdfskl";
        let dim1 = &file.dimension(dim1_name).expect("Could not find dimension");
        let dim2 = &file.dimension(dim2_name).expect("Could not find dimension");
        assert_eq!(dim1.len(), 10);
        assert_eq!(dim2.len(), 20);

        // verify variable data
        let var_name = "varstuff_int";
        let data_test: ArrayD<i32> = ArrayD::from_elem(ndarray::IxDyn(&[10, 20]), 42i32);
        let data_file = file
            .root()
            .variable(var_name)
            .expect("Could not find variable")
            .values::<i32>(None, None)
            .unwrap();
        assert_eq!(data_test.len(), data_file.len());
        assert_eq!(data_test, data_file);

        let var_name = "varstuff_float";
        let data_test = ArrayD::from_elem(ndarray::IxDyn(&[10]), 42.2f32);
        let data_file = file
            .root()
            .variable(var_name)
            .expect("Could not find variable")
            .values::<f32>(None, None)
            .unwrap();
        assert_eq!(data_test, data_file);

        // verify global attrs
        assert_eq!(
            AttrValue::Int(3),
            file.root()
                .attribute("testattr1")
                .expect("netcdf error")
                .expect("Could not find attribute")
                .value()
                .unwrap()
        );
        assert_eq!(
            AttrValue::Str("Global string attr".into()),
            file.root()
                .attribute("testattr2")
                .expect("netcdf error")
                .expect("Could not find attribute")
                .value()
                .unwrap()
        );

        // verify var attrs
        assert_eq!(
            AttrValue::Int(5),
            file.root()
                .variable(var_name)
                .expect("Could not find variable")
                .attribute("varattr1")
                .expect("netcdf error occured")
                .expect("Could not find attribute")
                .value()
                .unwrap()
        );
        assert_eq!(
            AttrValue::Str("Variable string attr".into()),
            file.root()
                .variable(var_name)
                .expect("Could not find variable")
                .attribute("varattr2")
                .expect("netcdf error occured")
                .expect("Could not find attribute")
                .value()
                .unwrap()
        );
    }
}

#[test]
fn all_var_types() {
    // write
    let d = tempfile::tempdir().unwrap();
    let name = "all_var_types.nc";
    {
        let f = d.path().join(name);
        let mut file = netcdf::create(&f).unwrap();

        let dim_name = "dim1";

        let mut root = file.root_mut();
        root.add_dimension(dim_name, 10).unwrap();

        // byte
        let data = vec![42i8; 10];
        let var_name = "var_byte";
        let mut var = root.add_variable::<i8>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        let data = vec![42u8; 10];
        let var_name = "var_char";
        let mut var = root.add_variable::<u8>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // short
        let data = vec![42i16; 10];
        let var_name = "var_short";
        let mut var = root.add_variable::<i16>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // ushort
        let data = vec![42u16; 10];
        let var_name = "var_ushort";
        let mut var = root.add_variable::<u16>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // int
        let data = vec![42i32; 10];
        let var_name = "var_int";
        let mut var = root.add_variable::<i32>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // uint
        let data = vec![42u32; 10];
        let var_name = "var_uint";
        let mut var = root.add_variable::<u32>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // int64
        let data = vec![42i64; 10];
        let var_name = "var_int64";
        let mut var = root.add_variable::<i64>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // uint64
        let data = vec![42u64; 10];
        let var_name = "var_uint64";
        let mut var = root.add_variable::<u64>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // float
        let data = vec![42.2f32; 10];
        let var_name = "var_float";
        let mut var = root.add_variable::<f32>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();

        // double
        let data = vec![42.2f64; 10];
        let var_name = "var_double";
        let mut var = root.add_variable::<f64>(var_name, &[dim_name]).unwrap();
        var.put_values(&data, None, None).unwrap();
    }

    {
        // read
        let f = d.path().join(name);
        let file = netcdf::open(f).unwrap();

        //byte
        let mut data = vec![0i8; 10];
        file.variable("var_byte")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42i8; 10], data);

        // ubyte
        let mut data = vec![0u8; 10];
        file.variable("var_char")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42u8; 10], data);

        // short
        let mut data = vec![0i16; 10];
        file.variable("var_short")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42i16; 10], data);

        // ushort
        let mut data = vec![0u16; 10];
        file.variable("var_ushort")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42u16; 10], data);

        // int
        let mut data = vec![0i32; 10];
        file.variable("var_int")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42i32; 10], data);

        // uint
        let mut data = vec![0u32; 10];
        file.variable("var_uint")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42u32; 10], data);

        // int64
        let mut data = vec![0i64; 10];
        file.variable("var_int64")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42i64; 10], data);

        // uint64
        let mut data = vec![0u64; 10];
        file.variable("var_uint64")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42u64; 10], data);

        // float
        let mut data = vec![0.0f32; 10];
        file.variable("var_float")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42.2f32; 10], data);

        // double
        let mut data = vec![0.0f64; 10];
        file.variable("var_double")
            .unwrap()
            .unwrap()
            .values_to(&mut data, None, None)
            .unwrap();
        assert_eq!(vec![42.2f64; 10], data);
    }
}

#[test]
#[cfg(feature = "ndarray")]
/// Tests the shape of a variable
/// when fetched using "Variable::as_array()"
fn fetch_ndarray() {
    let f = test_location().join("pres_temp_4D.nc");
    let file = netcdf::open(&f).unwrap();

    let pres = &file
        .variable("pressure")
        .unwrap()
        .expect("Could not find variable");
    let values_array = pres.values::<f64>(None, None).unwrap();
    assert_eq!(values_array.shape(), &[2, 2, 6, 12]);
}

#[test]
// test file modification
fn append() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("append.nc");
    let dim_name = "some_dimension";
    {
        // first creates a simple netCDF file
        // and create a variable called "some_variable" in it
        let mut file_w = netcdf::create(&f).unwrap();
        file_w.add_dimension(dim_name, 3).unwrap();
        let var = &mut file_w
            .add_variable::<i32>("some_variable", &[dim_name])
            .unwrap();
        var.put_values::<i32>(&[1, 2, 3], None, None).unwrap();
        // close it (done when `file_w` goes out of scope)
    }
    {
        // re-open it in append mode
        // and create a variable called "some_other_variable"
        let mut file_a = netcdf::append(&f).unwrap();
        let var = &mut file_a
            .add_variable::<i32>("some_other_variable", &[dim_name])
            .unwrap();
        var.put_values::<i32>(&[4, 5, 6], None, None).unwrap();
        // close it (done when `file_a` goes out of scope)
    }
    // finally open  the file in read only mode
    // and test the existence of both variable
    let file = netcdf::open(&f).unwrap();
    assert!(file
        .variables()
        .unwrap()
        .find(|x| x.as_ref().unwrap().name().unwrap() == "some_variable")
        .is_some());
    assert!(file
        .variables()
        .unwrap()
        .find(|x| x.as_ref().unwrap().name().unwrap() == "some_other_variable")
        .is_some());
}

#[test]
// test file modification
fn put_single_value() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("append_value.nc");
    let dim_name = "some_dimension";
    let var_name = "some_variable";
    {
        // first creates a simple netCDF file
        // and create a variable called "some_variable" in it
        let mut file_w = netcdf::create(&f).unwrap();
        file_w.add_dimension(dim_name, 3).unwrap();
        let var = &mut file_w.add_variable::<f32>(var_name, &[dim_name]).unwrap();
        var.put_values(&[1., 2., 3.], None, None).unwrap();
    }
    let indices: [usize; 1] = [0];
    {
        // re-open it in append mode
        let mut file_a = netcdf::append(&f).unwrap();
        let var = &mut file_a.variable_mut(var_name).unwrap();
        var.put_value(100.0f32, Some(&indices)).unwrap();
        // close it (done when `file_a` goes out of scope)
    }
    // finally open  the file in read only mode
    // and test the values of 'some_variable'
    let file = netcdf::open(&f).unwrap();
    let var = &file
        .variable(var_name)
        .unwrap()
        .expect("Could not find variable");
    assert_eq!(100.0, var.value(Some(&indices)).unwrap());
}

#[test]
// test file modification
fn put_values() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("append_values.nc");
    let dim_name = "some_dimension";
    let var_name = "some_variable";
    {
        // first creates a simple netCDF file
        // and create a variable called "some_variable" in it
        let mut file_w = netcdf::create(&f).unwrap();
        file_w.add_dimension(dim_name, 3).unwrap();
        let var = &mut file_w.add_variable::<i32>(var_name, &[dim_name]).unwrap();
        var.put_values(&[1i32, 2, 3], None, None).unwrap();
        // close it (done when `file_w` goes out of scope)
    }
    let indices = &[1];
    let values = &[100i32, 200];
    let len = &[values.len()];
    {
        // re-open it in append mode
        let mut file_a = netcdf::append(&f).unwrap();
        let var = &mut file_a.variable_mut(var_name).unwrap();
        let res = var.put_values(values, Some(indices), Some(len));
        assert_eq!(res.unwrap(), ());
        // close it (done when `file_a` goes out of scope)
    }
    // finally open  the file in read only mode
    // and test the values of 'some_variable'
    let file = netcdf::open(&f).unwrap();
    let var = &file
        .variable(var_name)
        .unwrap()
        .expect("Could not find variable");
    let mut d = vec![0i32; 3];
    var.values_to(d.as_mut_slice(), None, None).unwrap();
    assert_eq!(d, [1, 100, 200]);
}

#[test]
/// Test setting a fill value when creating a Variable
fn set_fill_value() {
    use netcdf::attribute::AttrValue;
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("fill_value.nc");
    let dim_name = "some_dimension";
    let var_name = "some_variable";
    let fill_value = -2 as i32;

    let mut file_w = netcdf::create(&f).unwrap();
    file_w.add_dimension(dim_name, 3).unwrap();
    let var = &mut file_w.add_variable::<i32>(var_name, &[dim_name]).unwrap();
    var.set_fill_value(fill_value).unwrap();

    var.put_values(&[2, 3], Some(&[1]), None).unwrap();

    let mut rvar = [0i32; 3];
    var.values_to(&mut rvar, None, None).unwrap();

    assert_eq!(rvar, [fill_value, 2, 3]);

    let var = &file_w
        .variable(var_name)
        .unwrap()
        .expect("Could not find variable");
    let attr = var
        .attribute("_FillValue")
        .expect("other error")
        .expect("could not find attribute")
        .value()
        .unwrap();
    // compare requested fill_value and attribute _FillValue
    assert_eq!(AttrValue::Int(fill_value), attr);

    let fill = var.fill_value::<i32>().unwrap();
    assert_eq!(fill, Some(fill_value));

    // Expecting an error when trying to get the wrong variable type
    var.fill_value::<f32>().unwrap_err();
}

#[test]
fn more_fill_values() {
    use netcdf::attribute::AttrValue;
    let d = tempfile::tempdir().expect("Could not create tempdir");
    let path = d.path().join("more_fill_values.nc");
    let mut file = netcdf::create(path).expect("Could not open file");

    file.add_dimension("x", 2).unwrap();
    let var = &mut file.add_variable::<i32>("v0", &["x"]).unwrap();
    var.set_fill_value(1_i32).unwrap();

    var.put_value(6, Some(&[1])).unwrap();
    assert_eq!(var.fill_value::<i32>().unwrap(), Some(1));

    assert_eq!(var.value::<i32>(Some(&[0])).unwrap(), 1_i32);
    assert_eq!(var.value::<i32>(Some(&[1])).unwrap(), 6_i32);

    let var = &mut file.add_variable::<i32>("v1", &["x"]).unwrap();
    unsafe {
        var.set_nofill().unwrap();
    }
    var.put_value(6, Some(&[1])).unwrap();
    assert_eq!(var.fill_value::<i32>().unwrap(), None);

    // assert_eq!(var.value::<i32>(Some(&[0])).unwrap(), GARBAGE);
    assert_eq!(var.value::<i32>(Some(&[1])).unwrap(), 6_i32);
    assert!(var.attribute("_FillValue").unwrap().is_none());

    let var = &mut file.add_variable::<i32>("v2", &["x"]).unwrap();
    var.set_fill_value(2_i32).unwrap();
    assert_eq!(
        var.attribute("_FillValue")
            .unwrap()
            .unwrap()
            .value()
            .unwrap(),
        AttrValue::Int(2)
    );
    var.set_fill_value(3_i32).unwrap();
    assert_eq!(
        var.attribute("_FillValue")
            .unwrap()
            .unwrap()
            .value()
            .unwrap(),
        AttrValue::Int(3)
    );
    unsafe { var.set_nofill().unwrap() };
    assert_eq!(var.fill_value::<i32>().unwrap(), None);

    var.put_value(6, Some(&[1])).unwrap();
    assert_eq!(var.fill_value::<i32>().unwrap(), None);

    // assert_eq!(var.value::<i32>(Some(&[0])).unwrap(), GARBAGE);
    assert_eq!(var.value::<i32>(Some(&[1])).unwrap(), 6_i32);

    // Following is the expected behaviour, but is not followed by netcdf
    // assert!(var.attribute("_FillValue").is_none());
}

#[test]
/// Test reading a slice of a variable into a buffer
fn read_slice_into_buffer() {
    let f = test_location().join("simple_xy.nc");
    let file = netcdf::open(&f).unwrap();
    let pres = &file
        .variable("data")
        .unwrap()
        .expect("Could not find variable");
    // pre-allocate the Array
    let mut values = vec![0i8; 6 * 3];
    let ind = &[0, 0];
    let len = &[6, 3];
    pres.values_to(values.as_mut_slice(), Some(ind), Some(len))
        .unwrap();
    let expected_values = [
        0i8, 1, 2, 12, 13, 14, 24, 25, 26, 36, 37, 38, 48, 49, 50, 60, 61, 62,
    ];
    for i in 0..values.len() {
        assert_eq!(expected_values[i], values[i]);
    }
}

#[test]
#[should_panic]
fn read_mismatched() {
    let f = test_location().join("simple_xy.nc");
    let file = netcdf::open(f).unwrap();

    let pres = &file.variable("data").unwrap().expect("variable not found");

    let mut d = vec![0; 40];
    pres.values_to(d.as_mut_slice(), None, Some(&[40, 1]))
        .unwrap();
}

#[test]
fn use_compression_chunking() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("compressed_var.nc");
    let mut file = netcdf::create(f).unwrap();

    file.add_dimension("x", 10).unwrap();

    let var = &mut file.add_variable::<i32>("compressed", &["x"]).unwrap();
    var.compression(5).unwrap();
    var.chunking(&[5]).unwrap();

    let v = vec![0i32; 10];
    var.put_values(&v, None, None).unwrap();

    let var = &mut file
        .add_variable::<i32>("compressed2", &["x", "x"])
        .unwrap();
    var.compression(9).unwrap();
    var.chunking(&[5, 5]).unwrap();
    var.put_values(&[1i32, 2, 3, 4, 5, 6, 7, 8, 9, 10], None, Some(&[10, 1]))
        .unwrap();

    let var = &mut file.add_variable::<i32>("chunked3", &["x"]).unwrap();
    assert!(
        if let netcdf::error::Error::SliceLen = var.chunking(&[2, 2]).unwrap_err() {
            true
        } else {
            false
        }
    );

    file.add_dimension("y", 0).unwrap();
    let var = &mut file.add_variable::<u8>("chunked4", &["y", "x"]).unwrap();

    var.chunking(&[100, 2]).unwrap();
}

#[test]
fn set_compression_all_variables_in_a_group() {
    let d = tempfile::tempdir().expect("Could not create tempdir");
    let path = d.path().join("set_compression_all_variables_in_a_group.nc");
    let mut file = netcdf::create(path).expect("Could not create file");

    file.add_dimension("x", 10)
        .expect("Could not create dimension");
    file.add_dimension("y", 15)
        .expect("Could not create dimension");
    file.add_variable::<u8>("var0", &["x", "y"])
        .expect("Could not create variable");
    file.add_variable::<u8>("var1", &["x", "y"])
        .expect("Could not create variable");
    file.add_variable::<u8>("var2", &["x", "y"])
        .expect("Could not create variable");
    file.add_variable::<u8>("var3", &["x", "y"])
        .expect("Could not create variable");

    for ref mut var in file.variables_mut().unwrap() {
        var.compression(9).expect("Could not set compression level");
    }

    let mut var = file.variable_mut("var0").unwrap();
    var.compression(netcdf_sys::NC_MAX_DEFLATE_LEVEL + 1)
        .unwrap_err();
}

#[test]
#[cfg(feature = "memory")]
fn read_from_memory() {
    use std::io::Read;
    let origfile = test_location().join("simple_xy.nc");
    let mut origfile = std::fs::File::open(origfile).unwrap();
    let mut bytes = Vec::new();
    origfile.read_to_end(&mut bytes).unwrap();

    let file = netcdf::open_mem(None, &bytes).unwrap();
    let x = &(*file).dimension("x").unwrap();
    assert_eq!(x.len(), 6);
    let y = &(*file).dimension("y").unwrap();
    assert_eq!(y.len(), 12);
    let mut v = vec![0i32; 6 * 12];
    (*file)
        .variable("data")
        .unwrap()
        .expect("Could not find variable")
        .values_to(&mut v, None, None)
        .unwrap();
    for (i, v) in v.iter().enumerate() {
        assert_eq!(*v, i as _);
    }
}

#[test]
fn add_conflicting_dimensions() {
    let d = tempfile::tempdir().unwrap();

    let mut file = netcdf::create(d.path().join("conflict_dim.nc")).unwrap();

    file.add_dimension("x", 10).unwrap();
    let e = file.add_dimension("x", 11).unwrap_err();
    assert!(match e {
        netcdf::error::Error::AlreadyExists => true,
        _ => false,
    });
    assert_eq!(file.dimension("x").unwrap().len(), 10);
}

#[test]
fn add_conflicting_variables() {
    let d = tempfile::tempdir().unwrap();
    let mut file = netcdf::create(d.path().join("conflict_var")).unwrap();

    file.add_dimension("x", 10).unwrap();
    file.add_dimension("y", 20).unwrap();

    file.add_variable::<i32>("x", &["x"]).unwrap();

    let e = file.add_variable::<f32>("x", &["y"]).unwrap_err();
    assert!(match e {
        netcdf::error::Error::AlreadyExists => {
            true
        }
        e => {
            panic!(e)
        }
    });
    assert_eq!(
        10,
        file.variable("x").unwrap().unwrap().dimensions()[0].len()
    );
}

#[test]
fn unlimited_dimension_single_putting() {
    let d = tempfile::tempdir().unwrap();
    let mut file = netcdf::create(d.path().join("unlim_single.nc")).unwrap();

    file.add_unlimited_dimension("x").unwrap();
    file.add_unlimited_dimension("y").unwrap();

    let var = &mut file.add_variable::<u8>("var", &["x", "y"]).unwrap();
    var.set_fill_value(0u8).unwrap();

    var.put_value(1, None).unwrap();
    assert_eq!(var.dimensions()[0].len(), 1);
    assert_eq!(var.dimensions()[1].len(), 1);
    var.put_value(2, Some(&[0, 1])).unwrap();
    assert_eq!(var.dimensions()[0].len(), 1);
    assert_eq!(var.dimensions()[1].len(), 2);
    var.put_value(3, Some(&[2, 0])).unwrap();
    assert_eq!(var.dimensions()[0].len(), 3);
    assert_eq!(var.dimensions()[1].len(), 2);

    let mut v = vec![0; 6];
    var.values_to(&mut v, None, Some(&[3, 2])).unwrap();

    assert_eq!(v, &[1, 2, 0, 0, 3, 0]);
}

fn check_equal<T>(var: &netcdf::Variable, check: &[T])
where
    T: netcdf::variable::Numeric
        + std::clone::Clone
        + std::default::Default
        + std::fmt::Debug
        + std::cmp::PartialEq,
{
    let mut v: Vec<T> = vec![Default::default(); check.len()];
    var.values_to(&mut v, None, None).unwrap();
    assert_eq!(v.as_slice(), check);
}

#[test]
fn unlimited_dimension_multi_putting() {
    let d = tempfile::tempdir().unwrap();
    let mut file = netcdf::create(d.path().join("unlim_multi.nc")).unwrap();

    file.add_unlimited_dimension("x").unwrap();
    file.add_unlimited_dimension("y").unwrap();
    file.add_dimension("z", 2).unwrap();
    file.add_unlimited_dimension("x2").unwrap();
    file.add_unlimited_dimension("x3").unwrap();
    file.add_unlimited_dimension("x4").unwrap();

    let var = &mut file.add_variable::<u8>("one_unlim", &["x", "z"]).unwrap();
    var.put_values(&[0u8, 1, 2, 3], None, None).unwrap();
    check_equal(var, &[0u8, 1, 2, 3]);
    var.put_values(&[0u8, 1, 2, 3, 4, 5, 6, 7], None, None)
        .unwrap();
    check_equal(var, &[0u8, 1, 2, 3, 4, 5, 6, 7]);

    let var = &mut file
        .add_variable::<u8>("unlim_first", &["z", "x2"])
        .unwrap();
    var.put_values(&[0u8, 1, 2, 3], None, None).unwrap();
    check_equal(var, &[0u8, 1, 2, 3]);
    var.put_values(&[0u8, 1, 2, 3, 4, 5, 6, 7], None, None)
        .unwrap();
    check_equal(var, &[0u8, 1, 2, 3, 4, 5, 6, 7]);

    let var = &mut file.add_variable::<u8>("two_unlim", &["x3", "x4"]).unwrap();
    var.set_fill_value(0u8).unwrap();
    let e = var.put_values(&[0u8, 1, 2, 3], None, None);
    assert!(if let netcdf::error::Error::Ambiguous = e.unwrap_err() {
        true
    } else {
        false
    });
    var.put_values(&[0u8, 1, 2, 3], None, Some(&[1, 4]))
        .unwrap();
    let mut v = vec![0; 4];
    var.values_to(&mut v, None, Some(&[1, 4])).unwrap();
    assert_eq!(v, &[0u8, 1, 2, 3]);
    var.put_values(&[4u8, 5, 6], None, Some(&[3, 1])).unwrap();

    let mut v = vec![0; 4 * 3];
    var.values_to(&mut v, None, Some(&[3, 4])).unwrap();

    assert_eq!(v, &[4, 1, 2, 3, 5, 0, 0, 0, 6, 0, 0, 0]);
}

#[test]
fn length_of_variable() {
    let d = tempfile::tempdir().unwrap();
    let mut file = netcdf::create(d.path().join("variable_length.nc")).unwrap();

    file.add_dimension("x", 4).unwrap();
    file.add_dimension("y", 6).unwrap();
    file.add_unlimited_dimension("z").unwrap();

    let var = &mut file.add_variable::<f32>("x", &["x", "y"]).unwrap();
    assert_eq!(var.len(), 4 * 6);

    let var = &mut file.add_variable::<f64>("z", &["x", "z"]).unwrap();
    var.put_value(1u8, Some(&[2, 8])).unwrap();
    assert_eq!(var.len(), 4 * 9);
}

#[test]
fn single_length_variable() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("single_length_variable.nc");
    let mut file = netcdf::create(&path).unwrap();

    let var = &mut file.add_variable::<u8>("x", &[]).unwrap();

    var.put_value(3u8, None).unwrap();
    assert_eq!(var.value::<u8>(Some(&[])).unwrap(), 3_u8);

    var.put_values::<u8>(&[], None, None).unwrap_err();
    assert_eq!(var.value::<u8>(None).unwrap(), 3_u8);

    var.put_values::<u8>(&[2, 3], None, None).unwrap_err();

    var.put_values::<u8>(&[6], None, None).unwrap();
    assert_eq!(var.value::<u8>(None).unwrap(), 6_u8);

    var.put_values::<u8>(&[8], Some(&[]), Some(&[])).unwrap();
    assert_eq!(var.value::<u8>(None).unwrap(), 8_u8);

    var.put_values::<u8>(&[10], Some(&[1]), None).unwrap_err();
    assert_eq!(var.value::<u8>(None).unwrap(), 8_u8);

    std::mem::drop(file);

    let file = netcdf::open(path).unwrap();

    let var = &file.variable("x").unwrap().unwrap();

    assert_eq!(var.value::<u8>(None).unwrap(), 8);
}

#[test]
fn put_then_def() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("put_then_def.nc");
    let mut file = netcdf::create(path).unwrap();

    let var = &mut file.add_variable::<i8>("x", &[]).unwrap();
    var.put_value(3i8, None).unwrap();

    let var2 = &mut file.add_variable::<i8>("y", &[]).unwrap();
    var2.put_value(4i8, None).unwrap();
}

#[test]
fn string_variables() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("string_variables.nc");
    {
        let mut file = netcdf::create(&path).unwrap();

        file.add_unlimited_dimension("x").unwrap();
        file.add_dimension("y", 2).unwrap();

        let var = &mut file.add_string_variable("str", &["x"]).unwrap();

        var.put_string("Hello world!", None).unwrap();
        var.put_string(
            "Trying a very long string just to see how that goes",
            Some(&[2]),
        )
        .unwrap();
        var.put_string("Foreign letters: ßæøå, #41&i1/99", Some(&[3]))
            .unwrap();

        // Some weird interaction between unlimited dimensions, put_str,
        // and the name of this variable leads to crash. This
        // can be observed by changing this     \ /    to "x"
        let var = &mut file.add_variable::<i32>("y", &[]).unwrap();
        var.put_value(42i32, Some(&[])).unwrap();
    }
    let file = netcdf::open(path).unwrap();

    let var = &file.variable("str").unwrap().unwrap();

    assert_eq!(var.string_value(Some(&[0])).unwrap(), "Hello world!");
    assert_eq!(var.string_value(Some(&[1])).unwrap(), "");
    assert_eq!(
        var.string_value(Some(&[2])).unwrap(),
        "Trying a very long string just to see how that goes"
    );
    assert_eq!(
        var.string_value(Some(&[3])).unwrap(),
        "Foreign letters: ßæøå, #41&i1/99"
    );

    let var = &file.variable("y").unwrap().unwrap();
    var.string_value(None).unwrap_err();
}

#[test]
fn unlimited_in_parents() {
    let d = tempfile::tempdir().expect("Could not create tempdir");
    let path = d.path().join("unlimited_in_parents.nc");
    {
        let mut file = netcdf::create(&path).unwrap();

        file.add_dimension("x", 0).unwrap();
        file.add_dimension("y", 0).unwrap();
        file.add_dimension("z0", 5).unwrap();
        let g = &mut file.add_group("g").unwrap();
        g.add_dimension("z1", 0).unwrap();
    }
    let mut file = netcdf::append(&path).unwrap();

    let g = &mut file.group_mut("g").unwrap();
    g.add_variable::<i16>("w", &["z1"]).unwrap();
    g.add_variable::<u16>("v", &["x"]).unwrap();
}

#[test]
fn dimension_identifiers() {
    let d = tempfile::tempdir().expect("Could not create tempdir");
    let path = d.path().join("dimension_identifiers.nc");
    {
        let mut file = netcdf::create(&path).unwrap();

        // Create groups and dimensions
        let dim = &file.add_dimension("x", 10).unwrap();
        let vrootid = dim.identifier();
        let g = &mut file.add_group("g").unwrap();
        let dim = &g.add_dimension("x", 5).unwrap();
        let vgid = dim.identifier();
        let mut gg = file.group_mut("g").unwrap().add_group("g").unwrap();
        let dim = &gg.add_dimension("x", 7).unwrap();
        let vggid = dim.identifier();

        // Create variables
        file.add_variable_from_identifiers::<i8>("v_self_id", &[vrootid])
            .unwrap();
        let mut g = file.group_mut("g").unwrap();
        g.add_variable_from_identifiers::<i8>("v_root_id", &[vrootid])
            .unwrap();
        g.add_variable_from_identifiers::<i8>("v_self_id", &[vgid])
            .unwrap();

        let gg = &mut g.group_mut("g").unwrap();
        gg.add_variable_from_identifiers::<i8>("v_root_id", &[vrootid])
            .unwrap();
        gg.add_variable_from_identifiers::<i8>("v_up_id", &[vgid])
            .unwrap();
        gg.add_variable_from_identifiers::<i8>("v_self_id", &[vggid])
            .unwrap();
    }

    let file = &netcdf::open(path).unwrap();

    assert_eq!(file.variable("v_self_id").unwrap().unwrap().len(), 10);
    assert_eq!(
        file.group("g")
            .unwrap()
            .variable("v_root_id")
            .unwrap()
            .len(),
        10
    );
    assert_eq!(
        file.group("g")
            .unwrap()
            .variable("v_self_id")
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        file.group("g")
            .unwrap()
            .group("g")
            .unwrap()
            .variable("v_self_id")
            .unwrap()
            .len(),
        7
    );
    assert_eq!(
        file.group("g")
            .unwrap()
            .group("g")
            .unwrap()
            .variable("v_up_id")
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        file.group("g")
            .unwrap()
            .group("g")
            .unwrap()
            .variable("v_root_id")
            .unwrap()
            .len(),
        10
    );
}

#[test]
/// Test setting/getting endian value when creating a Variable
fn set_get_endian() {
    use netcdf::variable::Endianness;
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("append.nc");
    let dim_name = "some_dimension";
    for i in &[Endianness::Little, Endianness::Big] {
        {
            // first creates a simple netCDF file
            // and create a variable called "some_variable" in it
            let mut file_w = netcdf::create(&f).unwrap();
            file_w.root_mut().add_dimension(dim_name, 3).unwrap();
            let var = &mut file_w
                .add_variable::<i32>("some_variable", &[dim_name])
                .unwrap();
            var.endian(*i).unwrap();
            assert_eq!(var.endian_value().unwrap(), *i);
            var.put_values::<i32>(&[1, 2, 3], None, None).unwrap();
            // close it (done when `file_w` goes out of scope)
        }
        {
            // re-open it
            // and get "some variable" endian_value
            let file_o = netcdf::open(&f).unwrap();
            let var = &file_o.variable("some_variable").unwrap().unwrap();
            assert_eq!(var.endian_value().unwrap(), *i);
            // close it (done when `file_a` goes out of scope)
        }
    }
}

mod strided {
    #[test]
    fn get_to_buffer() {
        let d = tempfile::tempdir().unwrap();
        let name = d.path().join("strided_buffer.nc");
        {
            let mut file = netcdf::create(&name).unwrap();
            file.add_dimension("z", 3).unwrap();
            file.add_dimension("y", 5).unwrap();
            file.add_dimension("x", 9).unwrap();
            let mut var = file.add_variable::<i32>("data", &["z", "y", "x"]).unwrap();
            let buffer = (0..3 * 5 * 9).collect::<Vec<_>>();
            var.put_values(&buffer, None, None).unwrap();
        }
        let file = netcdf::open(name).unwrap();
        let var = file.variable("data").unwrap().unwrap();

        let mut buffer = vec![0; 3 * 5 * 9];
        var.values_strided_to(&mut buffer, None, None, &[1, 1, 1])
            .unwrap();
        assert_eq!(&buffer, &(0..3 * 5 * 9).collect::<Vec<_>>());
        // Negative and zero strides seems not to be supported
        // var.values_strided_to(&mut buffer, None, None, &[0, 0, 0]).unwrap();
        // var.values_strided_to(&mut buffer, None, None, &[-1, -1, -1]).unwrap();
        let mut buffer = vec![0; 3 * 5 * 2];
        var.values_strided_to(&mut buffer, None, None, &[2, 1, 3])
            .unwrap();
        assert_eq!(
            buffer,
            &[
                0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 90, 93, 96, 99, 102, 105,
                108, 111, 114, 117, 120, 123, 126, 129, 132
            ]
        );

        let mut buffer = vec![0; 3 * 5 * 2];
        var.values_strided_to(&mut buffer, Some(&[0, 0, 0]), None, &[2, 1, 3])
            .unwrap();
        assert_eq!(
            buffer,
            &[
                0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 90, 93, 96, 99, 102, 105,
                108, 111, 114, 117, 120, 123, 126, 129, 132
            ]
        );

        let mut buffer = vec![0; 3 * 5 * 2];
        var.values_strided_to(&mut buffer, None, Some(&[2, 5, 3]), &[2, 1, 3])
            .unwrap();
        assert_eq!(
            buffer,
            &[
                0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 90, 93, 96, 99, 102, 105,
                108, 111, 114, 117, 120, 123, 126, 129, 132
            ]
        );

        let mut buffer = vec![0; 3 * 5 * 2];
        var.values_strided_to(&mut buffer, None, Some(&[2, 5, 4]), &[2, 1, 3])
            .unwrap_err();

        let mut buffer = vec![0; 3 * 5 * 2];
        let l = var
            .values_strided_to(&mut buffer, Some(&[2, 0, 4]), None, &[2, 1, 3])
            .unwrap();
        assert_eq!(
            &buffer[..l],
            &[94, 97, 103, 106, 112, 115, 121, 124, 130, 133]
        );

        let mut buffer = vec![0; 3 * 5 * 2];
        let l = var
            .values_strided_to(&mut buffer, Some(&[2, 0, 4]), Some(&[1, 2, 2]), &[2, 1, 3])
            .unwrap();
        assert_eq!(&buffer[..l], &[94, 97, 103, 106]);
    }
    #[test]
    fn put_buffer() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("put_strided_buffer.nc");
        let mut file = netcdf::create(path).unwrap();

        file.add_dimension("d0", 7).unwrap();
        file.add_dimension("d1", 9).unwrap();

        let mut var = file.add_variable::<i32>("var", &["d0", "d1"]).unwrap();

        let values = (0..7 * 9).collect::<Vec<i32>>();
        let zeros = [0; 7 * 9];
        var.put_values_strided(&values, None, None, &[1, 1])
            .unwrap();

        let mut buf = vec![0; 7 * 9];
        var.values_to(&mut buf, None, None).unwrap();
        assert_eq!(values, buf);

        var.put_values(&zeros, None, None).unwrap();
        var.put_values_strided(&values[1..][..12], Some(&[0, 0]), Some(&[4, 3]), &[2, 3])
            .unwrap();
        let mut buf = vec![0; 7 * 9];
        var.values_to(&mut buf, None, None).unwrap();
        assert_eq!(
            &buf,
            &vec![
                1, 0, 0, 2, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 5, 0, 0, 6, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 8, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10,
                0, 0, 11, 0, 0, 12, 0, 0,
            ]
        );
        var.put_values(&zeros, None, None).unwrap();
        var.put_values_strided(&values[1..], None, None, &[2, 3])
            .unwrap();
        let mut buf = vec![0; 7 * 9];
        var.values_to(&mut buf, None, None).unwrap();
        assert_eq!(
            &buf,
            &vec![
                1, 0, 0, 2, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 5, 0, 0, 6, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 8, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10,
                0, 0, 11, 0, 0, 12, 0, 0,
            ]
        );
        var.put_values(&zeros, None, None).unwrap();
        var.put_values_strided(&values[1..], None, None, &[3, 2])
            .unwrap();
        let mut buf = vec![0; 7 * 9];
        var.values_to(&mut buf, None, None).unwrap();
        assert_eq!(
            &buf,
            &vec![
                1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6,
                0, 7, 0, 8, 0, 9, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11,
                0, 12, 0, 13, 0, 14, 0, 15
            ]
        );
        var.put_values(&zeros, None, None).unwrap();
        var.put_values_strided(&values[1..], Some(&[2, 3]), None, &[3, 2])
            .unwrap();
        let mut buf = vec![0; 7 * 9];
        var.values_to(&mut buf, None, None).unwrap();
        assert_eq!(
            &buf,
            &vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 3, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 5, 0, 6, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0
            ]
        );
    }
}
