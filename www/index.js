import init, * as wasm from "./pkg/web_rdf_class_viz.js";

async function run() {
    await init();

    var network = null;

    // accepts a list of urls as arguments. Fetches each url
    // and extracts the file extension from the requst url.
    // Returns a list of objects with {'content': <content of the
    // url>, 'extension': <file extension of the url>}
    // Example:
    //
    function get_urls(urls) {
      return urls.map(url => {
        let extension = url.split('.').pop();
        let content = fetch(url).then(response => response.text());
        return {content, extension};
      });
    }

    let rdfvis = wasm.Visualizer.new();
    let brick_resp = get_urls(['https://raw.githubusercontent.com/open223/explore.open223.info/main/ontologies/Brick.ttl'])[0];
    brick_resp.content.then(content => {
        rdfvis.addOntology(content, brick_resp.extension);
    })
    .catch(err => console.log(err));
    console.log(rdfvis);



    //let resp = get_urls(['https://raw.githubusercontent.com/BrickSchema/Brick/master/examples/air_quality_sensors/air_quality_sensor_example.ttl'])[0];
    //resp.content.then(content => {
    //    console.log(content);
    //    console.log(vis.createDotFile(content, resp.extension));
    //})
    //.catch(err => console.log(err));

    // Function to render the graph using viz.js
    function renderGraph(dotString) {
        console.log(dotString);
        const graphContainer = document.getElementById('graphContainer');



        var parsedData = vis.parseDOTNetwork(dotString);

        var data = {
          nodes: parsedData.nodes,
          edges: parsedData.edges
        };
        var options = parsedData.options;

        options.interaction = {
          hover: true
        };
        options.physics = {
          barnesHut: {
              springLength: 200,
              avoidOverlap: .2,
              springConstant: .01
          },
        };
        // create a network
        network = new vis.Network(graphContainer, data, options);

        network.on("afterDrawing", function (ctx) {
            var dataURL = ctx.canvas.toDataURL();
            document.getElementById('canvasImg').href = dataURL;
          });
    }

    // Event listener for the submit button
    document.getElementById('submit').addEventListener('click', async () => {

        // input custom filter function
        const customFilterInput = document.getElementById('nodeFilter');
        const customFilter = customFilterInput.value;
        if (customFilter.length > 0) rdfvis.addFilter(new Function('from','to','edge', customFilter));

        // add the colors to the visualizer
        let color_map = document.getElementById('colorMap').value;
        if (color_map.length > 0) {
            color_map = JSON.parse(color_map);
            console.log(color_map);
            rdfvis.addClassColorMap(color_map);
        }

        // handle ontology files
        const ontologyFileInput = document.getElementById('ontologyFile');
        for (let file of ontologyFileInput.files) {
            console.log("Loading file " + file.name);
            const content = await file.text();
            rdfvis.addOntology(content, file.name.split('.').pop());
        }
        const ontologyUrlInput = document.getElementById('ontologyUrl');
        // parse the URLs out of the textfield, removing all empty lines
        const ontologyUrls = ontologyUrlInput.value.trim().split('\n').filter(url => url.length > 0);
        const ontologyUrlPromises = ontologyUrls.map(url => {
            console.log("Loading URL " + url);
            fetch(url).then(response => response.text())
        });
        const ontologyUrlContents = await Promise.all(ontologyUrlPromises);
        for (let content of ontologyUrlContents) {
            rdfvis.addOntology(content, 'ttl');
        }
        
        let dotString = '';

        // Handle file uploads
        const fileInput = document.getElementById('dataFile');
        if (fileInput.files.length > 0) {
            const filePromises = Array.from(fileInput.files).map(file => file.text());
            const fileContents = await Promise.all(filePromises);
            dotString = fileContents.map(content => rdfvis.createDotFile(content, 'ttl')).join('\n');
        }

        // Handle URLs
        const urlInput = document.getElementById('dataUrl');
        // parse the URLs out of the textfield, removing all empty lines
        const urls = urlInput.value.trim().split('\n').filter(url => url.length > 0);
        const urlPromises = urls.map(url => fetch(url).then(response => response.text()));
        const urlContents = await Promise.all(urlPromises);
        const urlDotStrings = urlContents.map(content => rdfvis.createDotFile(content, 'ttl')).join('\n');

        // Combine both file and URL dot strings
        dotString = `${dotString}\n${urlDotStrings}`;

        // hide d2text
        document.getElementById('d2text').style.display = 'none';
        // show graphContainer
        document.getElementById('graphContainer').style.display = 'block';

        // Render the graph
        renderGraph(dotString);
    });

    // Function to decode base64-encoded URL parameters
    function getBase64UrlParam(paramName) {
       const urlParams = new URLSearchParams(window.location.search);
       const encodedValue = urlParams.get(paramName);
       if (encodedValue) {
           return atob(encodedValue);
       }
       return '';
    }

    // Function to populate textareas with URL parameters
    function populateTextareas() {
       const ontologyUrlTextarea = document.getElementById('ontologyUrl');
       const dataUrlTextarea = document.getElementById('dataUrl');

       const ontologyUrls = getBase64UrlParam('ontologyUrls');
       const dataUrls = getBase64UrlParam('dataUrls');

       ontologyUrlTextarea.value = ontologyUrls;
       dataUrlTextarea.value = dataUrls;
    }

    // Populate textareas when the page loads
    window.addEventListener('load', populateTextareas);

    // disable the 'download' button until the graphContainer div contains something
    document.getElementById('download').disabled = true;

    // Function to generate D2 string and display it in the 'd2text' div
    // Event listener for the 'generated2' button
    document.getElementById('generated2').addEventListener('click', async () => {
        const dataFileInput = document.getElementById('dataFile');
        const dataUrlInput = document.getElementById('dataUrl');
        const d2text = document.getElementById('d2text');

        let dotString = '';

        // Handle file uploads
        if (dataFileInput.files.length > 0) {
            const filePromises = Array.from(dataFileInput.files).map(file => file.text());
            const fileContents = await Promise.all(filePromises);
            dotString = fileContents.map(content => rdfvis.create_d2_file(content, 'ttl')).join('\n');
        }

        // Handle URLs
        // parse the URLs out of the textfield, removing all empty lines
        const urls = dataUrlInput.value.trim().split('\n').filter(url => url.length > 0);
        const urlPromises = urls.map(url => fetch(url).then(response => response.text()));
        const urlContents = await Promise.all(urlPromises);
        const urlDotStrings = urlContents.map(content => rdfvis.create_d2_file(content, 'ttl')).join('\n');

        // Combine both file and URL dot strings
        let d2String = `${dotString}\n${urlDotStrings}`;

        console.log(d2String);


        // hide graphContainer
        document.getElementById('graphContainer').style.display = 'none';
        // show d2text
        document.getElementById('d2text').style.display = 'block';

        // Display the D2 string in the 'd2text' div
        d2text.textContent = d2String;
    });
}

run();