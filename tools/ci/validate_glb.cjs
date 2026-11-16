const fs=require('fs');const validator=require('gltf-validator');
validator.validateBytes(new Uint8Array(fs.readFileSync('terrain-audit.glb')),{uri:'terrain-audit.glb',maxIssues:50})
.then(report=>{report.worldcraft_source=process.env.GITHUB_SHA;fs.writeFileSync('export-validation.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report.issues));if(report.issues.numErrors)process.exitCode=1;})
.catch(error=>{console.error(error);process.exitCode=1;});
