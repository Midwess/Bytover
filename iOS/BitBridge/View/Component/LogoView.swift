//
//  LogoView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct LogoView: View {
    let width: CGFloat
    @State private var isVisible: Bool = false
    @EnvironmentObject var core: Core
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        VStack {
            if isVisible && !core.isSignedIn {
                LogoScene(gltfFileName: "Earth", logoScale: 1.1)
            }
        }
        .onChange(of: scenePhase, {_, newPhase in
            switch newPhase {
            case .background:
                isVisible = false
            default:
                isVisible = true
            }
        })
        .onAppear {
            isVisible = true
        }
        .onDisappear {
            isVisible = false
        }
    }
}

struct LogoView_Preview: PreviewProvider {
    static var previews: some View {
        LogoView(width: 200)
            .frame(width: 500, height: 500)
            .previewLayout(.sizeThatFits)
            .padding()
    }
}
