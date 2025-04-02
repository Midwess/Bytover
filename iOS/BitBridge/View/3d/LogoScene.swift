//
//  LogoScene.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI
import SceneKit
import GLTFSceneKit

struct LogoScene: UIViewRepresentable {
    public let gltfFileName: String
    public var logoScale: Float = 1.1
    public var rotation: SCNVector4?

    class Coordinator: NSObject {
        var parent: LogoScene
        weak var scene: SCNScene?
        
        init(_ parent: LogoScene) {
            self.parent = parent
            super.init()
        }
    }
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    func makeUIView(context: Context) -> SCNView {
        let sceneView = SCNView()
        
        sceneView.backgroundColor = .black.withAlphaComponent(0.0)
        sceneView.allowsCameraControl = false
        sceneView.antialiasingMode = .none
        sceneView.rendersContinuously = false
        sceneView.preferredFramesPerSecond = 15
        sceneView.isPlaying = false
        
        let materialCache: [String: SCNMaterial] = [
            "SeaTertiary": createMaterial(with: Theme.SeaTertiary.color),
            "BlueSky": createMaterial(with: Theme.BlueSky.color),
            "DarkBlue": createMaterial(with: Theme.DarkBlue.color),
            "Navy": createMaterial(with: Theme.Navy.color),
            "BlueViolet": createMaterial(with: Theme.BlueViolet.color),
            "LightSea": createMaterial(with: Theme.LightSea.color),
            "Orange": createMaterial(with: Theme.Orange.color)
        ]
        
        do {
            guard let url = Bundle.main.url(forResource: gltfFileName, withExtension: "gltf") else {
                print("Failed to find GLTF file: \(gltfFileName)")
                return sceneView
            }
            
            let sceneSource = GLTFSceneSource(url: url, options: [
                SCNSceneSource.LoadingOption.flattenScene: true,
                SCNSceneSource.LoadingOption.preserveOriginalTopology: false,
                SCNSceneSource.LoadingOption.createNormalsIfAbsent: true,
            ])
            let scene = try sceneSource.scene()
            context.coordinator.scene = scene
            
            configureEnvironmentMap(for: scene)
            
            sceneView.scene = scene
            
            let scale = SCNVector3(logoScale, logoScale, logoScale)
            scene.rootNode.scale = scale
            
            if let rotation = rotation {
                scene.rootNode.childNodes[0].rotation = rotation
            }
            
            applyMaterials(to: scene.rootNode, using: materialCache)
            
        } catch {
            print("Error loading GLTF: \(error.localizedDescription)")
        }
        
        if let rootNode = sceneView.scene?.rootNode {
            self.optimizeNodeHierarchy(rootNode)
        }
        
        return sceneView
    }
    
    func enableDrawingGroup() -> some View {
        self.drawingGroup()
    }
    
    func updateUIView(_ uiView: SCNView, context: Context) {}
    
    private func configureEnvironmentMap(for scene: SCNScene) {
        scene.background.contents = UIColor.clear
        
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.color = UIColor(white: 1.5, alpha: 1.0)
        ambientLight.light?.intensity = 100.0
        scene.rootNode.addChildNode(ambientLight)
        
        let directionalLight = SCNNode()
        directionalLight.light = SCNLight()
        directionalLight.light?.type = .directional
        directionalLight.light?.color = UIColor(white: 1.0, alpha: 1.0)
        directionalLight.eulerAngles = SCNVector3(x: -Float.pi/3, y: Float.pi/4, z: 0)
        scene.rootNode.addChildNode(directionalLight)
    }
    
    private func createMaterial(with color: Color) -> SCNMaterial {
        let material = SCNMaterial()
        material.diffuse.contents = UIColor(color)
        material.lightingModel = .phong
        material.locksAmbientWithDiffuse = true
        material.isDoubleSided = false
        material.writesToDepthBuffer = true
        material.readsFromDepthBuffer = true
        return material
    }
    
    private func applyMaterials(to rootNode: SCNNode, using materialCache: [String: SCNMaterial]) {
        rootNode.enumerateChildNodes { (node, _) in
            guard let geometry = node.geometry, let parentName = node.parent?.parent?.name else { return }
            
            switch parentName {
            case "Ocean", "Body", "Fins":
                geometry.materials = [materialCache["SeaTertiary"]!]
                
            case "Windows":
                geometry.materials = [materialCache["BlueSky"]!]
                
            case "Land", "Exhaust":
                geometry.materials = [materialCache["DarkBlue"]!]
                
            case "Head", "Screws", "Windows Frame":
                geometry.materials = [materialCache["Navy"]!]
                
            case "Trees", "Satelite Solar Panel", "Mountain", "Buildings":
                geometry.materials = [materialCache["BlueViolet"]!]
                
            case "Clouds", "Mountain Snowy Top", "Poles", "Satelite Body":
                geometry.materials = [materialCache["LightSea"]!]
                
            case "Satelite Solar Hinge", "Fire", "Scattered Fire":
                geometry.materials = [materialCache["Orange"]!]
                
            default:
                break
            }
        }
    }
    
    private func optimizeNodeHierarchy(_ rootNode: SCNNode) {
        rootNode.enumerateChildNodes { (node, _) in
            // Set reasonable culling distances
            node.renderingOrder = 0
            
            // Enable frustum culling
            node.categoryBitMask = 1
            
            // Simplify physics if not needed
            node.physicsBody = nil
            
            // Consolidate geometry when possible
            if let geometry = node.geometry {
                geometry.levelsOfDetail = nil
            }
        }
    }
}
