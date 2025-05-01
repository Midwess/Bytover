//
//  ShareView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes
import SceneKit

public struct ShareView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject var core: Core
    @State private var selectedResources: [SelectedResourceViewModel] = []
    @State private var isLoadingSelectedResource = false
    @State private var startTime = Date.now
    @State private var showShareModal = false

    public var body: some View {
        ZStack(alignment: .bottom) {
            StunningBackgroundGradient()
            ScrollView {
                LazyVStack(spacing: 8) {
                    LogoScene(gltfFileName: "Rocket", logoScale: 1.7)
                        .frame(width: screenSize.width, height: 100)
                        .overlay(Theme.gradientHeading
                            .opacity(0.5)
                            .blur(radius: 15)
                            .frame(width: .infinity, height: screenSize.width / 2)
                        )
                    
                    Text("Your dashboard")
                        .padding(.horizontal, 20)
                        .multilineTextAlignment(.center)
                        .modifier(Heading2())
                    
                    Spacer().frame(height: 10)
                    
                    UpgradePremiumButton()
                    
                    if selectedResources.isEmpty {
                        Spacer().frame(height: 150)
                    }
                    
                    ContentPickerView()
                        .padding(.trailing, SpaceTheme.screen.value - 10)
                    
                    ForEach(selectedResources, id: \.self.order_id) { item in
                        SelectedResourceItem(resource: item)
                            .padding(.horizontal, 15)
                            .id(item.order_id)
                    }
                    
                    if isLoadingSelectedResource {
                        VStack(alignment: .center, spacing: 5) {
                            ProgressView()
                                .scaleEffect(1.3)
                                .frame(width: 40, height: 40)
                            Text("Some media may need to download from iCloud")
                                .modifier(Label1())
                        }
                    }
                    
                    Spacer().frame(width: 10, height: 120)
                }
            }
            .mask(LinearGradient(gradient: Gradient(colors: [.black, .black, .black, .black, .clear]), startPoint: .top, endPoint: .bottom).opacity(0.9))
            .padding(.top, safeAreaInsets.top)
            .padding(.bottom, 150)
            
            ShareButton(width: 230)
                .padding(.bottom, safeAreaInsets.bottom + 16)
                .padding(.horizontal, SpaceTheme.screen.value)
        }
        .ignoresSafeArea()
        .task {
            await core.update(.transfer(.launch))
        }
        .onReceive(self.core.transfer, perform: { value in
            self.isLoadingSelectedResource = value?.is_loading_selected_resources ?? false
            if self.selectedResources.count != value?.selected_resources.count ?? 0 {
                self.selectedResources = value?.selected_resources ?? []
            }
        })
    }
}

#Preview {
   ShareView()
        .environmentObject(CoreMock.withSelectedFileTransfers())
}
