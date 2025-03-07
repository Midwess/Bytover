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
import CoreMotion

struct LogoScene: UIViewRepresentable {
    private let motionManager = CMMotionManager()
    public let gltfFileName: String
    public var logoScale: Float = 1.1
    public var rotation: SCNVector4?

    class Coordinator: NSObject {
        let maxMotion: Float = 0.3
        var parent: LogoScene
        var scene: SCNScene?
        
        init(_ parent: LogoScene) {
            self.parent = parent
            super.init()
            
            // Start motion updates with callback
//            parent.motionManager.deviceMotionUpdateInterval = 1.0 / 20.0
//            parent.motionManager.startDeviceMotionUpdates(using: .xArbitraryZVertical, to: .main) { [weak self] motion, error in
//                guard let motion = motion,
//                      let scene = self?.scene else { return }
//                
//                // Update scene rotation based on device motion
//                scene.rootNode.eulerAngles = SCNVector3(
//                    Float(max(-0.1, min(0.1, motion.attitude.pitch * 0.3)) * -1),
//                    Float(max(-0.2, min(0.2, motion.attitude.roll * 0.3)) * 1),
//                    Float(max(-0.2, min(0.2, motion.attitude.roll * 0.1)) * 1)
//                )
//            }
        }
    }
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    func makeUIView(context: Context) -> SCNView {
        let sceneView = SCNView()
        
        sceneView.backgroundColor = .black.withAlphaComponent(0.0)
        sceneView.allowsCameraControl = false
        
        do {
            let sceneSource = GLTFSceneSource(url: Bundle.main.url(forResource: gltfFileName, withExtension: "gltf")!, options: nil)
            let scene = try sceneSource.scene()
            context.coordinator.scene = scene
            
            // Configure environment map
            configureEnvironmentMap(for: scene)
            
            sceneView.scene = scene
            
            scene.rootNode.scale.x = logoScale
            scene.rootNode.scale.y = logoScale
            scene.rootNode.scale.z = logoScale
            
            if rotation != nil {
                scene.rootNode.childNodes[0].rotation.x = rotation!.x
                scene.rootNode.childNodes[0].rotation.y = rotation!.y
                scene.rootNode.childNodes[0].rotation.z = rotation!.z
            }
            
            scene.rootNode.enumerateChildNodes { (node, _) in
                if node.parent?.parent?.name == "Ocean" ||
                    node.parent?.parent?.name == "Body" ||
                    node.parent?.parent?.name == "Fins"
                {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.SeaTertiary.color)
                    node.geometry?.materials = [material]
                }
                else if
                    node.parent?.parent?.name == "Windows"  {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.BlueSky.color)
                    node.geometry?.materials = [material]
                    
                }
                else if node.parent?.parent?.name == "Land" ||
                            node.parent?.parent?.name == "Exhaust"
                {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.DarkBlue.color)
                    node.geometry?.materials = [material]
                }
                else if node.parent?.parent?.name == "Head" ||
                            node.parent?.parent?.name == "Screws" ||
                            node.parent?.parent?.name == "Windows Frame" {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.Navy.color)
                    node.geometry?.materials = [material]
                }
                else if node.parent?.parent?.name == "Trees" ||
                            node.parent?.parent?.name == "Satelite Solar Panel" ||
                            node.parent?.parent?.name == "Mountain" ||
                            node.parent?.parent?.name == "Buildings"
                {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.BlueViolet.color)
                    node.geometry?.materials = [material]
                }
                else if (node.parent?.parent?.name == "Clouds" ||
                         node.parent?.parent?.name == "Mountain Snowy Top" ||
                         node.parent?.parent?.name == "Poles" ||
                         node.parent?.parent?.name == "Satelite Body") {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.LightSea.color)
                    node.geometry?.materials = [material]
                }
                else if node.parent?.parent?.name == "Satelite Solar Hinge" ||
                            node.parent?.parent?.name == "Fire" ||
                            node.parent?.parent?.name == "Scattered Fire"
                {
                    let material = SCNMaterial()
                    material.diffuse.contents = UIColor(Theme.Orange.color)
                    node.geometry?.materials = [material]
                }
            }
        } catch {
            print("Error loading GLTF: \(error.localizedDescription)")
        }
        
        return sceneView
    }
    
    func updateUIView(_ uiView: SCNView, context: Context) {}
    
    static func dismantleUIView(_ uiView: SCNView, coordinator: Coordinator) {
        coordinator.parent.motionManager.stopDeviceMotionUpdates()
    }
    
    private func configureEnvironmentMap(for scene: SCNScene) {
        scene.background.contents = UIColor.clear
        
        // Add ambient light to ensure minimum lighting
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.color = UIColor(white: 1.5, alpha: 1.0)
        ambientLight.light?.intensity = 5000.0
        scene.rootNode.addChildNode(ambientLight)
        
        // Add directional light to simulate sun
        let directionalLight = SCNNode()
        directionalLight.light = SCNLight()
        directionalLight.light?.type = .directional
        directionalLight.light?.color = UIColor(white: 1.0, alpha: 1.0)
        directionalLight.eulerAngles = SCNVector3(x: -Float.pi/3, y: Float.pi/4, z: 0)
        ambientLight.light?.intensity = 100.0
        scene.rootNode.addChildNode(directionalLight)
    }
}
